//! The virtual pointer sink on the bound Hyprland instance's Wayland socket,
//! connected only after its peer is proven to be the bound compositor.

use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;

use agent_desktop_core::{AdapterError, Deadline, ErrorCode, Point};
use wayland_client::protocol::{wl_output, wl_pointer, wl_seat};
use wayland_protocols_wlr::virtual_pointer::v1::client::zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1;
use wayland_protocols_wlr::virtual_pointer::v1::client::zwlr_virtual_pointer_v1::ZwlrVirtualPointerV1;

use super::motion_request::MotionRequest;
use super::output_point::OutputPoint;
use super::pointer_devices::PointerDevices;
use super::pointer_sink::PointerSink;
use super::pointer_state::PointerState;
use super::wheel_axis::WheelAxis;
use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;
use crate::system::hyprland::clients::client_read::read_cursor;
use crate::system::hyprland::toplevel::bound_socket::{connect_bound, unadvertised};
use crate::system::wayland::capture::frame_freshness::monotonic_now;
use crate::system::wayland::link::Link;

const MANAGER: &str = "zwlr_virtual_pointer_manager_v1";
const MANAGER_VERSION: u32 = 2;
const OUTPUT_VERSION: u32 = 4;
const WHEEL_STEP: f64 = 15.0;
const NEED: &str = "pointer input on Hyprland";
const RELEASE_SETTLE_MS: u64 = 1_000;
const ONE_LOGICAL_PIXEL: u32 = 120;

/// Reads the compositor's cursor in global logical pixels.
pub(crate) type CursorRead = Box<dyn FnMut() -> Result<Point, AdapterError> + Send>;

/// Sends pointer frames through one virtual pointer per output. Event times
/// come from the monotonic clock, so presses of separate commands are never
/// counted as one multi-click.
pub(crate) struct LivePointerSink {
    link: Link<PointerState>,
    state: PointerState,
    devices: PointerDevices,
    socket: PathBuf,
    current: Option<String>,
    pressed_on: Vec<(u32, String)>,
    #[allow(dead_code)]
    cursor: CursorRead,
}

impl LivePointerSink {
    pub(crate) fn open(
        binding: &HyprlandBinding,
        deadline: Deadline,
    ) -> Result<Self, AdapterError> {
        let bound = binding.clone();
        Self::over(
            connect_bound(binding)?,
            binding.wayland_socket.clone(),
            deadline,
            Box::new(move || read_cursor(&bound, deadline)),
        )
    }

    /// The sink over a stream already proven to reach the bound compositor.
    fn over(
        stream: UnixStream,
        socket: PathBuf,
        deadline: Deadline,
        cursor: CursorRead,
    ) -> Result<Self, AdapterError> {
        let fault =
            |fault: crate::system::wayland::wayland_fault::WaylandFault| fault.into_error(&socket);
        let mut state = PointerState::default();
        let mut link = Link::over(stream, &mut state, deadline).map_err(fault)?;
        let exchange = &state.exchange;
        let manager_global = exchange
            .find(MANAGER, MANAGER_VERSION)
            .cloned()
            .ok_or_else(|| unadvertised(&format!("{MANAGER} version {MANAGER_VERSION}"), NEED))?;
        let seat_global = exchange
            .find("wl_seat", 1)
            .cloned()
            .ok_or_else(|| unadvertised("wl_seat", NEED))?;
        let output_globals: Vec<_> = exchange.all("wl_output").cloned().collect();
        let manager: ZwlrVirtualPointerManagerV1 = link.bind(&manager_global, MANAGER_VERSION, ());
        let seat: wl_seat::WlSeat = link.bind(&seat_global, 1, ());
        let mut bound: Vec<(u32, wl_output::WlOutput)> = Vec::new();
        for global in &output_globals {
            bound.push((global.name, link.bind(global, OUTPUT_VERSION, global.name)));
        }
        link.roundtrip(&mut state).map_err(fault)?;
        let outputs = bound
            .into_iter()
            .filter_map(|(registry, output)| {
                state
                    .outputs
                    .iter()
                    .find(|(name, _)| *name == registry)
                    .map(|(_, name)| (name.clone(), output))
            })
            .collect();
        Ok(Self {
            link,
            state,
            devices: PointerDevices {
                seat,
                manager,
                outputs,
                pointers: Vec::new(),
            },
            socket,
            current: None,
            pressed_on: Vec::new(),
            cursor,
        })
    }

    /// Waits, past the run's deadline, until the compositor has read the
    /// releases. A client that exits after a bare flush can lose them:
    /// libwayland-server destroys a hung-up client before reading what it
    /// sent last. The wait stays under the signal release's own.
    fn settle_release(&mut self) -> bool {
        Deadline::detached_after(RELEASE_SETTLE_MS)
            .is_ok_and(|deadline| self.link.roundtrip_by(&mut self.state, deadline).is_ok())
    }

    fn flush(&self) -> Result<(), AdapterError> {
        self.link
            .flush()
            .map_err(|fault| fault.into_error(&self.socket))
    }

    fn current_pointer(&self) -> Result<ZwlrVirtualPointerV1, AdapterError> {
        self.current
            .as_deref()
            .and_then(|output| self.devices.made(output))
            .cloned()
            .ok_or_else(no_motion)
    }
}

fn no_motion() -> AdapterError {
    AdapterError::new(
        ErrorCode::Internal,
        "A pointer event came before any motion",
    )
}

fn now_ms() -> u32 {
    (monotonic_now().as_millis() % (u128::from(u32::MAX) + 1)) as u32
}

impl PointerSink for LivePointerSink {
    fn motion(&mut self, point: &OutputPoint) -> Result<(), AdapterError> {
        let handle = self.link.handle();
        let (pointer, made) = self
            .devices
            .pointer_for(&point.output, &handle)
            .ok_or_else(|| {
                AdapterError::new(
                    ErrorCode::InvalidArgs,
                    format!("The compositor names no output '{}'", point.output),
                )
            })?;
        let pointer = pointer.clone();
        if made {
            self.link
                .roundtrip(&mut self.state)
                .map_err(|fault| fault.into_error(&self.socket))?;
        }
        for request in motion_requests(point.x, self.current.is_none()) {
            match request {
                MotionRequest::StillRelative => pointer.motion(now_ms(), 0.0, 0.0),
                MotionRequest::Absolute { x } => {
                    pointer.motion_absolute(now_ms(), x, point.y, point.x_extent, point.y_extent)
                }
                MotionRequest::Frame => pointer.frame(),
            }
        }
        self.current = Some(point.output.clone());
        self.flush()
    }

    fn button(&mut self, code: u32, down: bool) -> Result<(), AdapterError> {
        let output = match down {
            true => self.current.clone(),
            false => self
                .pressed_on
                .iter()
                .find(|(pressed, _)| *pressed == code)
                .map(|(_, output)| output.clone())
                .or_else(|| self.current.clone()),
        };
        let pointer = output
            .as_deref()
            .and_then(|output| self.devices.made(output))
            .cloned()
            .ok_or_else(no_motion)?;
        let state = if down {
            wl_pointer::ButtonState::Pressed
        } else {
            wl_pointer::ButtonState::Released
        };
        pointer.button(now_ms(), code, state);
        pointer.frame();
        match (down, output) {
            (true, Some(output)) => self.pressed_on.push((code, output)),
            (false, _) => self.pressed_on.retain(|(pressed, _)| *pressed != code),
            _ => {}
        }
        self.flush()
    }

    fn wheel(&mut self, axis: WheelAxis, discrete: i32) -> Result<(), AdapterError> {
        let pointer = self.current_pointer()?;
        let axis = match axis {
            WheelAxis::Vertical => wl_pointer::Axis::VerticalScroll,
            WheelAxis::Horizontal => wl_pointer::Axis::HorizontalScroll,
        };
        pointer.axis_source(wl_pointer::AxisSource::Wheel);
        pointer.axis_discrete(now_ms(), axis, WHEEL_STEP * f64::from(discrete), discrete);
        pointer.frame();
        self.flush()
    }

    fn pause(&mut self, duration: Duration) {
        std::thread::sleep(duration);
    }

    fn recover(&mut self, buttons: &[u32]) -> bool {
        let sent = buttons
            .iter()
            .map(|code| self.button(*code, false).is_ok())
            .filter(|sent| !*sent)
            .count()
            == 0;
        sent && self.settle_release()
    }

    fn finish(&mut self) -> Result<(), AdapterError> {
        self.link
            .roundtrip(&mut self.state)
            .map_err(|fault| fault.into_error(&self.socket))
    }

    fn settle(&mut self) -> bool {
        self.settle_release()
    }
}

/// The requests for a motion to `x`, starting one pixel beside it when
/// `first`. Each absolute motion follows a still relative one in its frame:
/// Hyprland forwards an absolute virtual pointer motion with no relative
/// motion, XWayland then moves its absolute pointer device while it sends the
/// wheel through its relative one, and the switch makes GDK3 reset its scroll
/// baseline, so the next notch scrolls nothing (probes/linux/FINDINGS.md L44-1).
fn motion_requests(x: u32, first: bool) -> Vec<MotionRequest> {
    let aims = match first {
        true => vec![one_pixel_beside(x), x],
        false => vec![x],
    };
    aims.into_iter()
        .flat_map(|x| {
            [
                MotionRequest::StillRelative,
                MotionRequest::Absolute { x },
                MotionRequest::Frame,
            ]
        })
        .collect()
}

fn beside_dx(output_x: u32) -> i32 {
    match output_x >= ONE_LOGICAL_PIXEL {
        true => -1,
        false => 1,
    }
}

/// Hyprland skips a motion to the pixel the cursor already holds, and 0.56.2
/// gives no pointer focus to a window mapped under a still cursor while it
/// fades in. A first motion that starts one logical pixel away makes the
/// compositor hit-test the aim, so the input reaches the window under it.
fn one_pixel_beside(x: u32) -> u32 {
    match x.checked_sub(ONE_LOGICAL_PIXEL) {
        Some(left) => left,
        None => x.saturating_add(ONE_LOGICAL_PIXEL),
    }
}

#[cfg(test)]
#[path = "live_pointer_sink_tests.rs"]
mod tests;
