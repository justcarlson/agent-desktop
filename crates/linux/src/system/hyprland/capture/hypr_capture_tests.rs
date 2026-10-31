use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use agent_desktop_core::{
    AdapterError, DisplayInfo, ErrorCode, ProcessId, Rect, ScreenshotTarget, WindowInfo,
    WindowState,
};

use super::*;
use crate::system::hyprland::clients::client_workspace::ClientWorkspace;
use crate::system::hyprland::clients::hypr_client::HyprClient;
use crate::system::hyprland::identity::bound_toplevel::BoundToplevel;
use crate::system::wayland::capture::composition::Composition;
use crate::system::wayland::capture::shm_layout::XRGB8888;
use crate::system::wayland::toplevel::toplevel_view::ToplevelView;

struct FakeWorld {
    enforced: bool,
    bound: Result<BoundToplevel, AdapterError>,
    composition: Result<Composition, AdapterError>,
    frame: Result<RawFrame, AdapterError>,
    still: bool,
    displays: VecDeque<Vec<DisplayInfo>>,
    log: Log,
}

type Log = Rc<RefCell<Vec<String>>>;

fn note(log: &Log, entry: impl Into<String>) {
    log.borrow_mut().push(entry.into());
}

impl CaptureWorld for FakeWorld {
    fn enforced(&mut self) -> Result<bool, AdapterError> {
        note(&self.log, "enforced");
        Ok(self.enforced)
    }

    fn bind(&mut self, _win: &WindowInfo) -> Result<BoundToplevel, AdapterError> {
        note(&self.log, "bind");
        self.bound.clone()
    }

    fn composition(&mut self, bound: &BoundToplevel) -> Result<Composition, AdapterError> {
        note(
            &self.log,
            format!("composition {:#x}", bound.client.address),
        );
        self.composition.clone()
    }

    fn capture_toplevel(
        &mut self,
        identifier: &str,
        composition: Composition,
    ) -> Result<RawFrame, AdapterError> {
        note(&self.log, format!("toplevel {identifier} {composition:?}"));
        self.frame.clone()
    }

    fn still_bound(&mut self, bound: &BoundToplevel) -> Result<bool, AdapterError> {
        note(&self.log, format!("still {:#x}", bound.client.address));
        Ok(self.still)
    }

    fn displays(&mut self) -> Result<Vec<DisplayInfo>, AdapterError> {
        note(&self.log, "displays");
        let now = self.displays.front().cloned().unwrap_or_default();
        if self.displays.len() > 1 {
            self.displays.pop_front();
        }
        Ok(now)
    }

    fn capture_output(&mut self, name: &str) -> Result<RawFrame, AdapterError> {
        note(&self.log, format!("output {name}"));
        self.frame.clone()
    }
}

struct FakeHider {
    log: Log,
    hide: Result<(), AdapterError>,
    show: Result<(), AdapterError>,
}

impl CursorHider for FakeHider {
    fn hide(&mut self) -> Result<(), AdapterError> {
        note(&self.log, "hide");
        self.hide.clone()
    }

    fn show(&mut self) -> Result<(), AdapterError> {
        note(&self.log, "show");
        self.show.clone()
    }
}

fn frame(width: u32, height: u32) -> RawFrame {
    RawFrame {
        data: vec![0x40; (width * height * 4) as usize],
        format: XRGB8888,
        width,
        height,
        stride: width * 4,
        inverted: false,
    }
}

fn bound() -> BoundToplevel {
    BoundToplevel {
        client: HyprClient {
            address: 0x10,
            pid: 7,
            title: "Notes".into(),
            class: "app".into(),
            geometry: Rect {
                x: 0.0,
                y: 0.0,
                width: 400.0,
                height: 300.0,
            },
            workspace: ClientWorkspace {
                name: "special:hid".into(),
                monitor: None,
            },
            shown: false,
        },
        toplevel: ToplevelView {
            identifier: "abc".into(),
            address: 0x10,
            activated: false,
            wlr_index: Some(0),
        },
    }
}

fn display(id: &str, bounds: Rect, scale: f64, is_primary: bool) -> DisplayInfo {
    DisplayInfo {
        id: id.into(),
        bounds,
        is_primary,
        scale,
    }
}

fn layout() -> Vec<DisplayInfo> {
    vec![
        display(
            "Virtual-1",
            Rect {
                x: 0.0,
                y: 0.0,
                width: 64.0,
                height: 32.0,
            },
            1.0,
            true,
        ),
        display(
            "Virtual-2",
            Rect {
                x: -48.0,
                y: -40.0,
                width: 48.0,
                height: 40.0,
            },
            1.25,
            false,
        ),
    ]
}

fn world(frame: RawFrame) -> FakeWorld {
    FakeWorld {
        enforced: false,
        bound: Ok(bound()),
        composition: Ok(Composition::Stale),
        frame: Ok(frame),
        still: true,
        displays: VecDeque::from([layout()]),
        log: Log::default(),
    }
}

fn entries(world: &FakeWorld) -> Vec<String> {
    world.log.borrow().clone()
}

fn window() -> WindowInfo {
    WindowInfo {
        id: "w-hypr-50-60-abc".into(),
        title: "Notes".into(),
        app: "app".into(),
        pid: ProcessId::new(7),
        process_instance: Some("boot:100".into()),
        bounds: None,
        state: WindowState::default(),
    }
}

fn screen(world: &mut FakeWorld, target: ScreenshotTarget) -> Result<ImageBuffer, AdapterError> {
    let mut hider = FakeHider {
        log: world.log.clone(),
        hide: Ok(()),
        show: Ok(()),
    };
    capture_screen(world, &mut hider, &target)
}

#[test]
fn an_enforcing_compositor_is_refused_before_anything_is_bound_or_captured() {
    let mut world = world(frame(800, 600));
    world.enforced = true;
    let error = capture_window(&mut world, &window()).unwrap_err();
    assert_eq!(error.code, ErrorCode::PermDenied);
    let error = screen(&mut world, ScreenshotTarget::FullScreen).unwrap_err();
    assert_eq!(error.code, ErrorCode::PermDenied);
    assert_eq!(entries(&world), ["enforced", "enforced"]);
}

#[test]
fn the_window_is_captured_from_its_own_toplevel_at_its_buffer_scale() {
    let mut world = world(frame(800, 600));
    let image = capture_window(&mut world, &window()).unwrap();
    assert_eq!(
        entries(&world),
        [
            "enforced",
            "bind",
            "composition 0x10",
            "toplevel abc Stale",
            "still 0x10"
        ]
    );
    assert_eq!(
        (image.width, image.height, image.scale_factor),
        (800, 600, 2.0)
    );
}

#[test]
fn a_composed_window_is_captured_as_current_and_any_other_as_stale() {
    for composition in [Composition::Current, Composition::Stale] {
        let mut world = world(frame(800, 600));
        world.composition = Ok(composition);
        capture_window(&mut world, &window()).unwrap();
        assert_eq!(entries(&world)[3], format!("toplevel abc {composition:?}"));
    }
}

#[test]
fn a_composition_that_cannot_be_read_captures_nothing() {
    let mut world = world(frame(800, 600));
    world.composition = Err(AdapterError::internal("no clients"));
    let error = capture_window(&mut world, &window()).unwrap_err();
    assert_eq!(error.message, "no clients");
    assert_eq!(entries(&world), ["enforced", "bind", "composition 0x10"]);
}

#[test]
fn a_window_that_changed_during_capture_returns_no_pixels() {
    let mut world = world(frame(800, 600));
    world.still = false;
    let error = capture_window(&mut world, &window()).unwrap_err();
    assert_eq!(error.code, ErrorCode::WindowNotFound);
}

#[test]
fn a_window_that_binds_to_nothing_is_never_captured() {
    let mut world = world(frame(800, 600));
    world.bound = Err(AdapterError::new(ErrorCode::WindowNotFound, "gone"));
    let error = capture_window(&mut world, &window()).unwrap_err();
    assert_eq!(error.code, ErrorCode::WindowNotFound);
    assert_eq!(entries(&world), ["enforced", "bind"]);
}

#[test]
fn a_frame_error_is_returned_as_it_is() {
    let mut world = world(frame(800, 600));
    world.frame = Err(AdapterError::timeout("no frame"));
    let error = capture_window(&mut world, &window()).unwrap_err();
    assert_eq!(error.code, ErrorCode::Timeout);
    assert_eq!(
        entries(&world),
        ["enforced", "bind", "composition 0x10", "toplevel abc Stale"]
    );
}

#[test]
fn the_selected_display_is_captured_by_name_at_its_scale_between_hide_and_show() {
    let mut world = world(frame(60, 50));
    let image = screen(&mut world, ScreenshotTarget::Screen(1)).unwrap();
    assert_eq!(
        entries(&world),
        [
            "enforced",
            "displays",
            "hide",
            "output Virtual-2",
            "show",
            "displays"
        ]
    );
    assert_eq!(
        (image.width, image.height, image.scale_factor),
        (60, 50, 1.25)
    );
}

#[test]
fn full_screen_is_the_primary_display() {
    let mut world = world(frame(64, 32));
    let image = screen(&mut world, ScreenshotTarget::FullScreen).unwrap();
    assert!(entries(&world).contains(&"output Virtual-1".to_owned()));
    assert_eq!(image.scale_factor, 1.0);
}

#[test]
fn a_failed_hide_skips_the_capture() {
    let mut world = world(frame(64, 32));
    let mut hider = FakeHider {
        log: world.log.clone(),
        hide: Err(AdapterError::internal("hide")),
        show: Ok(()),
    };
    let error = capture_screen(&mut world, &mut hider, &ScreenshotTarget::FullScreen).unwrap_err();
    assert_eq!(error.message, "hide");
    assert_eq!(entries(&world), ["enforced", "displays", "hide"]);
    assert!(
        !entries(&world)
            .iter()
            .any(|entry| entry.starts_with("output"))
    );
}

#[test]
fn show_runs_after_a_failed_capture_and_its_own_failure_is_reported() {
    let mut world = world(frame(64, 32));
    world.frame = Err(AdapterError::timeout("no frame"));
    let error = screen(&mut world, ScreenshotTarget::FullScreen).unwrap_err();
    assert_eq!(error.code, ErrorCode::Timeout);
    assert!(entries(&world).ends_with(&["show".to_owned()]));
    let mut world = self::world(frame(64, 32));
    let mut hider = FakeHider {
        log: world.log.clone(),
        hide: Ok(()),
        show: Err(AdapterError::internal("show")),
    };
    let error = capture_screen(&mut world, &mut hider, &ScreenshotTarget::FullScreen).unwrap_err();
    assert_eq!(error.message, "show");
}

#[test]
fn a_display_that_changed_during_capture_returns_no_pixels() {
    let mut world = world(frame(64, 32));
    let mut moved = layout();
    moved[0].bounds.x = 10.0;
    world.displays = VecDeque::from([layout(), moved]);
    let error = screen(&mut world, ScreenshotTarget::FullScreen).unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidArgs);
}

#[test]
fn a_frame_whose_size_is_not_the_display_at_its_scale_is_refused() {
    let mut world = world(frame(48, 40));
    let error = screen(&mut world, ScreenshotTarget::Screen(1)).unwrap_err();
    assert_eq!(error.code, ErrorCode::ActionFailed);
    let mut world = self::world(frame(61, 50));
    assert!(screen(&mut world, ScreenshotTarget::Screen(1)).is_ok());
    let mut world = self::world(frame(62, 50));
    assert!(screen(&mut world, ScreenshotTarget::Screen(1)).is_err());
}

#[test]
fn a_display_target_that_no_longer_matches_is_refused_before_capture() {
    let mut world = world(frame(64, 32));
    let mut expected = layout()[0].clone();
    expected.scale = 2.0;
    let error = screen(&mut world, ScreenshotTarget::Display { index: 0, expected }).unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidArgs);
    assert!(!entries(&world).iter().any(|entry| entry == "hide"));
}
