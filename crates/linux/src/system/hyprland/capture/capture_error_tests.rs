use agent_desktop_core::ErrorCode;

use super::*;

const SOCKET: &str = "/run/user/1000/wayland-1";

fn code(fault: CaptureFault) -> ErrorCode {
    capture_error(&fault, SOCKET).code
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
            CaptureFault::Failed(FrameFailure::Stopped),
            ErrorCode::WindowNotFound,
        ),
        (
            CaptureFault::Failed(FrameFailure::Unknown),
            ErrorCode::ActionFailed,
        ),
        (
            CaptureFault::Failed(FrameFailure::BufferConstraints),
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
fn a_lost_compositor_names_its_socket() {
    let error = capture_error(&CaptureFault::FrameGone, SOCKET);
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
    let error = capture_error(
        &CaptureFault::Unsupported("zwlr_screencopy_manager_v1"),
        SOCKET,
    );
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
