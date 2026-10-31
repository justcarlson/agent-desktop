use std::cell::{Cell, RefCell};
use std::path::PathBuf;

use agent_desktop_core::{AdapterError, Deadline, DeliverySemantics, ErrorCode, ScreenshotTarget};

use super::super::*;
use crate::system::capture::window::fakes::*;
use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;

fn binding() -> HyprlandBinding {
    HyprlandBinding {
        signature: "sig".into(),
        ipc_socket: PathBuf::from("/run/user/1000/hypr/sig/.socket.sock"),
        wayland_socket: PathBuf::from("/run/user/1000/wayland-1"),
        pid: 50,
        start_time: 60,
    }
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

fn no_hyprland(
    _: &HyprlandBinding,
    _: &ScreenshotTarget,
    _: Deadline,
) -> Result<ImageBuffer, AdapterError> {
    panic!("this request reached Hyprland capture")
}

fn bound() -> Result<Option<HyprlandBinding>, AdapterError> {
    Ok(Some(binding()))
}

#[test]
fn every_hyprland_display_goes_to_hyprland_capture_and_never_to_the_portal() {
    for wanted in [ScreenshotTarget::FullScreen, ScreenshotTarget::Screen(1)] {
        let seen = RefCell::new(Vec::new());
        let hyprland = |binding: &HyprlandBinding, given: &ScreenshotTarget, deadline: Deadline| {
            seen.borrow_mut()
                .push((binding.pid, label(given), deadline.remaining()));
            Err(AdapterError::timeout("stub"))
        };
        let backends = WaylandBackends {
            binding: &bound,
            portal: &no_portal,
            hyprland: &hyprland,
        };
        let rig = Rig::standard();
        let command = Deadline::after(2_000).expect("a deadline");
        let wanted_label = label(&wanted);
        let error = capture_through(&rig.env(), &Route::Wayland, wanted, command, &backends)
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
fn a_refused_hyprland_binding_is_the_error_and_reaches_no_backend() {
    let refused = || {
        Err(AdapterError::new(
            ErrorCode::PlatformNotSupported,
            "refused",
        ))
    };
    let backends = WaylandBackends {
        binding: &refused,
        portal: &no_portal,
        hyprland: &no_hyprland,
    };
    let rig = Rig::standard();
    let error = capture_through(
        &rig.env(),
        &Route::Wayland,
        ScreenshotTarget::FullScreen,
        deadline(),
        &backends,
    )
    .expect_err("refused");
    assert_eq!(error.message, "refused");
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
}

#[test]
fn a_window_on_a_bound_hyprland_is_refused_before_the_binding_is_read() {
    let looked = Cell::new(false);
    let lookup = || {
        looked.set(true);
        bound()
    };
    let backends = WaylandBackends {
        binding: &lookup,
        portal: &no_portal,
        hyprland: &no_hyprland,
    };
    let rig = Rig::standard();
    let error = capture_through(
        &rig.env(),
        &Route::Wayland,
        ScreenshotTarget::ExactWindow(target()),
        deadline(),
        &backends,
    )
    .expect_err("refused");
    assert_eq!(error.code, ErrorCode::PlatformNotSupported);
    assert!(!looked.get());
}
