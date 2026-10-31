use std::path::Path;

use agent_desktop_core::ErrorCode;
use wayland_client::ConnectError;

use super::*;

const SOCKET: &str = "/run/user/1000/wayland-1";

#[test]
fn a_silent_compositor_is_a_retryable_timeout_that_names_its_socket() {
    let error = WaylandFault::Silent.into_error(Path::new(SOCKET));
    assert_eq!(error.code, ErrorCode::Timeout);
    let details = error.details.expect("details");
    assert_eq!(details["display_unavailable"], "silent");
    assert_eq!(details["retryable"], true);
    assert_eq!(details["detail"], SOCKET);
}

#[test]
fn a_compositor_that_hung_up_is_a_missing_server_and_not_retried() {
    let error = WaylandFault::Gone.into_error(Path::new(SOCKET));
    assert_eq!(error.code, ErrorCode::PlatformNotSupported);
    let details = error.details.expect("details");
    assert_eq!(details["display_unavailable"], "missing");
    assert_eq!(details["retryable"], false);
    assert!(
        details["detail"]
            .as_str()
            .is_some_and(|detail| detail.contains(SOCKET))
    );
}

#[test]
fn a_protocol_error_is_internal_and_keeps_the_compositor_text() {
    let fault = WaylandFault::Protocol("wl_registry#2: invalid version".to_owned());
    let error = fault.into_error(Path::new(SOCKET));
    assert_eq!(error.code, ErrorCode::Internal);
    assert_eq!(
        error.platform_detail.as_deref(),
        Some("wl_registry#2: invalid version")
    );
}

#[test]
fn a_client_library_that_cannot_load_is_unsupported_and_names_the_library() {
    let fault = WaylandFault::from_connect(ConnectError::NoWaylandLib);
    assert_eq!(fault, WaylandFault::NoLibrary);
    let error = fault.into_error(Path::new(SOCKET));
    assert_eq!(error.code, ErrorCode::PlatformNotSupported);
    let detail = error.platform_detail.expect("platform detail");
    assert!(detail.contains("libwayland-client.so.0"), "{detail}");
    assert!(detail.contains(SOCKET), "{detail}");
}

#[test]
fn a_connection_without_a_compositor_is_gone() {
    assert_eq!(
        WaylandFault::from_connect(ConnectError::NoCompositor),
        WaylandFault::Gone
    );
    assert_eq!(
        WaylandFault::from_connect(ConnectError::InvalidFd),
        WaylandFault::Gone
    );
}
