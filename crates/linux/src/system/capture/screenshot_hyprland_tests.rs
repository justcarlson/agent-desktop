use std::cell::RefCell;
use std::path::PathBuf;

use agent_desktop_core::{AdapterError, Deadline, DeliverySemantics, ErrorCode, ScreenshotTarget};

use super::super::*;
use crate::system::capture::window::fakes::*;
use crate::system::hyprland::hyprland_binding::HyprlandBinding;

fn hyprland_route() -> Route {
    Route::Wayland(WaylandShell::Hyprland(HyprlandBinding {
        signature: "sig".into(),
        ipc_socket: PathBuf::from("/run/user/1000/hypr/sig/.socket.sock"),
        wayland_socket: PathBuf::from("/run/user/1000/wayland-1"),
        pid: 50,
        start_time: 60,
    }))
}

fn label(target: &ScreenshotTarget) -> String {
    match target {
        ScreenshotTarget::ExactWindow(window) => format!("window {}", window.id),
        ScreenshotTarget::FullScreen => "full".into(),
        ScreenshotTarget::Screen(index) => format!("screen {index}"),
        ScreenshotTarget::Display { index, .. } => format!("display {index}"),
    }
}

fn no_portal(_: &ScreenshotTarget, _: Deadline) -> Result<ImageBuffer, AdapterError> {
    panic!("Hyprland reached the portal")
}

#[test]
fn every_hyprland_target_goes_to_hyprland_capture_and_never_to_the_portal() {
    let targets = [
        ScreenshotTarget::ExactWindow(target()),
        ScreenshotTarget::FullScreen,
        ScreenshotTarget::Screen(1),
    ];
    for wanted in targets {
        let seen = RefCell::new(Vec::new());
        let hyprland = |binding: &HyprlandBinding, given: &ScreenshotTarget, deadline: Deadline| {
            seen.borrow_mut()
                .push((binding.pid, label(given), deadline.remaining()));
            Err(AdapterError::timeout("stub"))
        };
        let backends = WaylandBackends {
            portal: &no_portal,
            hyprland: &hyprland,
        };
        let rig = Rig::standard();
        let command = Deadline::after(2_000).expect("a deadline");
        let wanted_label = label(&wanted);
        let error = capture_through(&rig.env(), &hyprland_route(), wanted, command, &backends)
            .expect_err("the stub fails");
        assert_eq!(error.code, ErrorCode::Timeout);
        assert_eq!(error.disposition, DeliverySemantics::not_delivered());
        let seen = seen.into_inner();
        assert_eq!(seen.len(), 1);
        assert_eq!((seen[0].0, &seen[0].1), (50, &wanted_label));
        assert!(
            seen[0].2 <= std::time::Duration::from_millis(2_000),
            "{:?}",
            seen[0].2
        );
        assert_eq!(rig.x11_touched(), 0);
    }
}

#[test]
fn the_hyprland_debug_frame_is_the_window_capture() {
    let seen = RefCell::new(None);
    let hyprland = |_: &HyprlandBinding, given: &ScreenshotTarget, _: Deadline| {
        seen.replace(Some(label(given)));
        Err(AdapterError::new(ErrorCode::WindowNotFound, "stub"))
    };
    let backends = WaylandBackends {
        portal: &no_portal,
        hyprland: &hyprland,
    };
    let rig = Rig::standard();
    let error = capture_frame_through(
        &rig.env(),
        &hyprland_route(),
        &target(),
        deadline(),
        &backends,
    )
    .expect_err("the stub fails");
    assert_eq!(error.code, ErrorCode::WindowNotFound);
    assert_eq!(
        seen.into_inner(),
        Some(label(&ScreenshotTarget::ExactWindow(target())))
    );
    assert_eq!(rig.x11_touched(), 0);
}
