use std::path::Path;

use agent_desktop_core::ErrorCode;

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
