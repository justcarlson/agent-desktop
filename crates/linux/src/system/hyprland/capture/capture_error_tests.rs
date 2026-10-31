use std::path::Path;

use agent_desktop_core::ErrorCode;

use super::*;
use crate::system::wayland::wayland_fault::WaylandFault;

const SOCKET: &str = "/run/user/1000/wayland-1";

fn error_for(fault: &CaptureFault) -> AdapterError {
    capture_error(fault, Path::new(SOCKET))
}

fn code(fault: CaptureFault) -> ErrorCode {
    error_for(&fault).code
}

#[test]
fn each_fault_maps_to_its_documented_code() {
    let rows = [
        (
            CaptureFault::Setup(WaylandFault::Silent),
            ErrorCode::Timeout,
        ),
        (
            CaptureFault::Setup(WaylandFault::Gone),
            ErrorCode::PlatformNotSupported,
        ),
        (CaptureFault::FrameGone, ErrorCode::PlatformNotSupported),
        (CaptureFault::FrameSilent, ErrorCode::Timeout),
        (
            CaptureFault::Unsupported("wl_shm"),
            ErrorCode::ActionNotSupported,
        ),
        (
            CaptureFault::Setup(WaylandFault::Protocol("bad".into())),
            ErrorCode::Internal,
        ),
        (
            CaptureFault::Failed(FrameFailure::Stopped),
            ErrorCode::WindowNotFound,
        ),
        (
            CaptureFault::Failed(FrameFailure::BufferConstraints),
            ErrorCode::ActionFailed,
        ),
        (
            CaptureFault::Failed(FrameFailure::Unknown),
            ErrorCode::ActionFailed,
        ),
        (CaptureFault::NoFormat, ErrorCode::ActionFailed),
        (CaptureFault::Stale, ErrorCode::ActionFailed),
        (
            CaptureFault::OutputMissing("DP-1".to_owned()),
            ErrorCode::InvalidArgs,
        ),
    ];
    for (fault, expected) in rows {
        assert_eq!(code(fault.clone()), expected, "{fault:?}");
    }
}

#[test]
fn a_silent_compositor_names_its_socket_and_is_retryable() {
    let error = error_for(&CaptureFault::Setup(WaylandFault::Silent));
    let details = error.details.unwrap_or_default();
    assert_eq!(details["detail"], SOCKET);
    assert_eq!(details["retryable"], true);
}

#[test]
fn a_lost_compositor_names_its_socket() {
    let error = error_for(&CaptureFault::FrameGone);
    assert!(error.message.contains(SOCKET), "{}", error.message);
    assert_eq!(
        error
            .details
            .as_ref()
            .map(|d| d["display_unavailable"].clone()),
        Some("missing".into())
    );
}

#[test]
fn a_missing_global_is_named() {
    let error = error_for(&CaptureFault::Unsupported("zwlr_screencopy_manager_v1"));
    assert!(
        error.message.contains("zwlr_screencopy_manager_v1"),
        "{}",
        error.message
    );
}

#[test]
fn the_permission_refusal_is_perm_denied_and_names_the_option() {
    let error = permission_refusal();
    assert_eq!(error.code, ErrorCode::PermDenied);
    assert!(error.message.contains(ENFORCE_OPTION));
    assert_eq!(
        error.details.as_ref().map(|d| d["option"].clone()),
        Some(ENFORCE_OPTION.into())
    );
}

#[test]
fn a_missed_frame_suggests_no_flag_screenshot_lacks() {
    let error = error_for(&CaptureFault::FrameSilent);
    let suggestion = error.suggestion.unwrap_or_default();
    assert!(!suggestion.contains("--timeout"), "{suggestion}");
    assert!(!suggestion.is_empty());
}

#[test]
fn a_refused_frame_names_the_reason_the_compositor_gave() {
    let rows = [
        (FrameFailure::BufferConstraints, "buffer_constraints"),
        (FrameFailure::Unknown, "unknown"),
    ];
    for (failure, reason) in rows {
        let error = error_for(&CaptureFault::Failed(failure));
        let details = error.details.unwrap_or_default();
        assert_eq!(details["kind"], "capture_refused", "{failure:?}");
        assert_eq!(details["reason"], reason, "{failure:?}");
    }
}
