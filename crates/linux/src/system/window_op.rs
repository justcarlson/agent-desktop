use std::time::Duration;

use agent_desktop_core::{
    AdapterError, Deadline, DeliverySemantics, ErrorCode, InteractionLease, OperationReport,
    OperationTarget, Rect, WindowInfo, WindowOp,
};

use self::hold::{give_back_place, hold_place, is_resize, remember_place};
use self::refuse::{no_client, refused, unreadable, unsupported_session, write_failed};
use self::window_place::PlaceStore;
use super::display_server::{DisplayServer, session_server};
use super::focus_cause::Cause;
use super::focus_world::LiveWorld as FrameWorld;
use super::proc_stat;
use super::process_facts::{self, Lookup};
use super::restore_minimized::{Lifted, lift, still_hidden};
use super::window_client::{ClientWorld, client_of};
use super::window_op_shadow::{Shadow, to_client, to_visible};
use super::window_op_verdict::{
    POLL, POSITION_TOLERANCE, SETTLE_BUDGET, Verdict, judge, observed_of, requested, unverified,
};
use super::x11::control::{ControlError, ControlRead, xwayland_clients};
use super::x11::control_send::{WindowWrite, write_window};
use super::x11::stacking_source::StackingSource;
use super::x11::window_facts::{WindowFacts, read_facts};
use super::x11::{DisplayReader, X11Client, X11State};
use super::{SharedBus, list_displays};
use crate::tree::Extents;

const CONFIRM_PAUSE: Duration = Duration::from_millis(120);
const NUDGE: i32 = 40;

/// What a window operation reads and writes, so the decision can be driven by
/// a fixed desktop.
trait OpWorld: ClientWorld {
    fn lookup(&self, pid: u32) -> Lookup;
    fn stopped(&self, pid: u32) -> bool;
    fn clients(&self) -> ControlRead<Vec<X11Client>>;
    fn facts(&self, client: u32) -> ControlRead<WindowFacts>;
    fn write(&self, client: u32, write: &WindowWrite) -> Result<(), ControlError>;
    fn lift(&self, client: u32) -> Result<Lifted, AdapterError>;
    fn displays(&self) -> Vec<Rect>;
    fn on_wayland(&self) -> bool;
    fn remember(&self, win: &WindowInfo, client: u32, rectangle: Option<Extents>);
    fn remembered(&self, win: &WindowInfo, client: u32) -> Option<Extents>;
}

struct Live<'a> {
    frame: FrameWorld<'a>,
    deadline: Deadline,
}

impl ClientWorld for Live<'_> {
    fn global_position(&self) -> bool {
        self.frame.global_position()
    }
}

impl OpWorld for Live<'_> {
    fn lookup(&self, pid: u32) -> Lookup {
        process_facts::lookup(pid)
    }

    fn stopped(&self, pid: u32) -> bool {
        proc_stat::is_stopped(pid)
    }

    fn clients(&self) -> ControlRead<Vec<X11Client>> {
        match session_server() {
            DisplayServer::X11(_) => match DisplayReader.clients_within(self.deadline) {
                X11State::Ready { clients, .. } => ControlRead::Ready(clients),
                X11State::NoWindowManager => ControlRead::NoManager,
                X11State::Unavailable => ControlRead::Unavailable,
            },
            DisplayServer::Wayland => xwayland_clients(self.deadline),
            DisplayServer::Unavailable(..) => ControlRead::Unavailable,
        }
    }

    fn on_wayland(&self) -> bool {
        session_server() == DisplayServer::Wayland
    }

    fn facts(&self, client: u32) -> ControlRead<WindowFacts> {
        read_facts(client, self.deadline)
    }

    fn write(&self, client: u32, write: &WindowWrite) -> Result<(), ControlError> {
        write_window(client, write, self.deadline)
    }

    fn lift(&self, client: u32) -> Result<Lifted, AdapterError> {
        lift(client, self.deadline)
    }

    fn remember(&self, win: &WindowInfo, client: u32, rectangle: Option<Extents>) {
        let (Some(store), Some(instance)) = (
            PlaceStore::in_runtime_dir(),
            win.process_instance.as_deref(),
        ) else {
            return;
        };
        match rectangle {
            Some(place) => {
                let alive = |pid: u32, instance: &str| matches!(process_facts::lookup(pid), Lookup::Found(facts) if facts.instance == instance);
                let _ = store.save(client, win.pid.get(), instance, place, &alive);
            }
            None => store.forget(client, instance),
        }
    }

    fn remembered(&self, win: &WindowInfo, client: u32) -> Option<Extents> {
        PlaceStore::in_runtime_dir()?.load(client, win.process_instance.as_deref()?)
    }

    fn displays(&self) -> Vec<Rect> {
        list_displays(self.deadline)
            .map(|displays| displays.into_iter().map(|display| display.bounds).collect())
            .unwrap_or_default()
    }
}

fn same_instance(world: &impl OpWorld, win: &WindowInfo) -> Option<bool> {
    match world.lookup(win.pid.get()) {
        Lookup::Found(facts) => Some(win.process_instance.as_deref() == Some(&facts.instance)),
        Lookup::Gone => Some(false),
        Lookup::Unreadable => None,
    }
}

fn ensure_owned_before(world: &impl OpWorld, win: &WindowInfo) -> Result<(), AdapterError> {
    if win.process_instance.is_none() {
        return Err(AdapterError::stale_ref_because(
            "A window operation needs the process instance the window was listed with",
        )
        .with_disposition(DeliverySemantics::not_delivered()));
    }
    if same_instance(world, win) == Some(false) {
        return Err(AdapterError::stale_ref_because(
            "The window belongs to a different process instance or the process has exited",
        )
        .with_disposition(DeliverySemantics::not_delivered()));
    }
    Ok(())
}

fn ensure_owned_after(world: &impl OpWorld, win: &WindowInfo) -> Result<(), AdapterError> {
    if same_instance(world, win) == Some(false) {
        return Err(AdapterError::new(
            ErrorCode::WindowNotFound,
            "The window's process changed or exited during the operation",
        )
        .with_suggestion("Run 'list-windows' to find the window again")
        .with_disposition(DeliverySemantics::delivered_unverified()));
    }
    Ok(())
}

fn locate(world: &impl OpWorld, win: &WindowInfo) -> Result<(u32, Shadow), AdapterError> {
    let read = world.clients();
    let ControlRead::Ready(clients) = &read else {
        if world.on_wayland() {
            return Err(refused(
                ErrorCode::ActionNotSupported,
                "wayland_session",
                "A Wayland session lets no client move, resize, minimize or maximize another client's window",
                "Run the desktop session on X11 with a window manager, or operate the window yourself",
            ));
        }
        return Err(unsupported_session(&read));
    };
    let id = client_of(world, win, clients).map_err(|cause| {
        if cause == Cause::WindowNotMapped && world.on_wayland() {
            unsupported_session(&ControlRead::NotX11)
        } else {
            no_client(cause)
        }
    })?;
    let shadow = clients
        .iter()
        .find(|client| client.id == id)
        .map_or((0, 0, 0, 0), |client| client.shadow);
    Ok((id, shadow))
}

fn settle(
    world: &impl OpWorld,
    client: u32,
    write: &WindowWrite,
    deadline: Deadline,
) -> (Verdict, Option<WindowFacts>) {
    let limit = deadline.capped(SETTLE_BUDGET);
    loop {
        let mut observed = world.facts(client).ready();
        let at_limit = limit.is_expired();
        let verdict = match observed {
            Some(facts) => judge(write, &facts, at_limit),
            None if at_limit => Verdict::Mismatch,
            None => Verdict::Pending,
        };
        match verdict {
            Verdict::Satisfied if matches!(write, WindowWrite::MoveResize { .. }) => {
                if limit.remaining() < CONFIRM_PAUSE {
                    return (Verdict::Mismatch, observed);
                }
                std::thread::sleep(CONFIRM_PAUSE);
                observed = world.facts(client).ready();
                match observed {
                    Some(facts) if judge(write, &facts, true) == Verdict::Satisfied => {
                        return (Verdict::Satisfied, Some(facts));
                    }
                    _ if limit.is_expired() => return (Verdict::Mismatch, observed),
                    _ => {}
                }
            }
            Verdict::Pending => {}
            _ => return (verdict, observed),
        }
        std::thread::sleep(POLL.min(limit.remaining()));
    }
}

fn apply(
    world: &impl OpWorld,
    win: &WindowInfo,
    client: u32,
    write: &WindowWrite,
    deadline: Deadline,
) -> Result<(), AdapterError> {
    world.write(client, write).map_err(write_failed)?;
    let (verdict, observed) = settle(world, client, write, deadline);
    ensure_owned_after(world, win)?;
    match verdict {
        Verdict::Satisfied => Ok(()),
        Verdict::Pending | Verdict::Mismatch => {
            let displays = world.displays();
            Err(unverified(write, observed.as_ref(), &displays))
        }
    }
}

fn read_before(world: &impl OpWorld, client: u32) -> Result<WindowFacts, AdapterError> {
    match world.facts(client) {
        ControlRead::Ready(facts) => Ok(facts),
        _ => Err(unreadable(DeliverySemantics::not_delivered())),
    }
}

fn restore(
    world: &impl OpWorld,
    win: &WindowInfo,
    client: u32,
    deadline: Deadline,
) -> Result<(), AdapterError> {
    let mut current = read_before(world, client)?;
    if current.hidden && world.on_wayland() {
        return Err(refused(
            ErrorCode::ActionNotSupported,
            "wayland_session",
            "A Wayland compositor refuses to show a minimized window for another client",
            "Show the window yourself, for example from the overview",
        ));
    }
    if current.hidden {
        if let Lifted::StillHidden { manager } = world.lift(client)? {
            return Err(still_hidden(&manager));
        }
        current = read_before(world, client)
            .map_err(|error| error.with_disposition(DeliverySemantics::delivered_unverified()))?;
    }
    if !current.maximized_vertical && !current.maximized_horizontal {
        return give_back_place(world, win, client, deadline);
    }
    apply(
        world,
        win,
        client,
        &WindowWrite::SetMaximized(false),
        deadline,
    )?;
    give_back_place(world, win, client, deadline)
}

fn reobserve(world: &impl OpWorld, client: u32, mut error: AdapterError) -> AdapterError {
    let (ControlRead::Ready(facts), Some(observed)) = (
        world.facts(client),
        error
            .details
            .as_mut()
            .and_then(|details| details.get_mut("observed"))
            .filter(|observed| observed.is_object()),
    ) else {
        return error;
    };
    *observed = observed_of(Some(&facts));
    error
}

fn write_and_hold(
    world: &impl OpWorld,
    win: &WindowInfo,
    client: u32,
    write: WindowWrite,
    deadline: Deadline,
) -> Result<(), AdapterError> {
    if write == WindowWrite::SetMaximized(true) {
        remember_place(world, win, client);
    }
    let before = match world.facts(client) {
        ControlRead::Ready(facts) if is_resize(&write) => Some(facts.client),
        _ => None,
    };
    let outcome = apply(world, win, client, &write, deadline);
    let held = hold_place(world, win, client, before, deadline);
    let outcome = match (outcome, before) {
        (Err(error), Some(_)) => Err(reobserve(world, client, error)),
        (outcome, _) => outcome,
    };
    outcome?;
    held
}

fn dispatch(
    world: &impl OpWorld,
    win: &WindowInfo,
    op: &WindowOp,
    deadline: Deadline,
) -> Result<(), AdapterError> {
    let write = requested(op)?;
    ensure_owned_before(world, win)?;
    if world.stopped(win.pid.get()) {
        return Err(AdapterError::app_unresponsive(&win.app)
            .with_disposition(DeliverySemantics::not_delivered()));
    }
    let (client, shadow) = locate(world, win)?;
    if deadline.is_expired() {
        return Err(deadline
            .timeout_error()
            .with_disposition(DeliverySemantics::not_delivered()));
    }
    let done = match write {
        Some(write) => write_and_hold(world, win, client, to_client(write, shadow), deadline),
        None => restore(world, win, client, deadline),
    };
    done.map_err(|error| to_visible(error, shadow))
}

fn run(
    world: &impl OpWorld,
    win: &WindowInfo,
    op: &WindowOp,
    deadline: Deadline,
) -> Result<OperationReport, AdapterError> {
    let dispatched = dispatch(world, win, op, deadline);
    OperationReport::from_window_dispatch(
        win,
        op,
        super::window_report::window_readback(op),
        dispatched,
    )
}

pub(crate) fn window_op(
    bus: &SharedBus,
    win: &WindowInfo,
    op: WindowOp,
    lease: &InteractionLease,
) -> Result<OperationReport, AdapterError> {
    OperationTarget::for_window(win)?;
    let deadline = lease.deadline();
    let frame = FrameWorld::new(bus, win, deadline)
        .map_err(|error| error.with_disposition(DeliverySemantics::not_delivered()))?;
    run(&Live { frame, deadline }, win, &op, deadline)
}

#[path = "window_op_hold.rs"]
mod hold;
#[path = "window_op_refuse.rs"]
mod refuse;
#[path = "window_place.rs"]
mod window_place;

#[cfg(test)]
#[path = "window_op_tests.rs"]
mod tests;
