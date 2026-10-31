use agent_desktop_core::ErrorCode;

use super::*;

fn reason(error: &AdapterError) -> (&str, bool) {
    let details = error.details.as_ref().expect("details");
    (
        details["display_unavailable"].as_str().expect("reason"),
        details["retryable"].as_bool().expect("retryable"),
    )
}

#[test]
fn ambiguity_and_mismatch_are_permanent_with_their_own_reasons() {
    let ambiguous = BindRefusal::Ambiguous {
        signatures: vec!["a".into(), "b".into()],
    }
    .to_error();
    assert_eq!(ambiguous.code, ErrorCode::PlatformNotSupported);
    assert_eq!(reason(&ambiguous), ("hyprland_ambiguous", false));
    assert_eq!(
        ambiguous.details.as_ref().expect("details")["signatures"],
        serde_json::json!(["a", "b"])
    );
    let mismatch = BindRefusal::Mismatch {
        detail: "swapped".into(),
    }
    .to_error();
    assert_eq!(mismatch.code, ErrorCode::PlatformNotSupported);
    assert_eq!(reason(&mismatch), ("hyprland_mismatch", false));
}

#[test]
fn a_silent_compositor_socket_is_a_retryable_timeout_naming_the_socket() {
    let silent = BindRefusal::Silent {
        socket: PathBuf::from("/run/user/1000/hypr/x/.socket.sock"),
    }
    .to_error();
    assert_eq!(silent.code, ErrorCode::Timeout);
    assert_eq!(reason(&silent), ("silent", true));
    assert_eq!(
        silent.details.as_ref().expect("details")["detail"],
        "/run/user/1000/hypr/x/.socket.sock"
    );
}
