use agent_desktop_core::{DeliverySemantics, ErrorCode};

use super::HideRefusal;

#[test]
fn every_refusal_is_not_delivered_with_its_kind() {
    let cases = [
        (
            HideRefusal::Timeout { pid: 7 },
            ErrorCode::Timeout,
            "cursor_hide_timeout",
        ),
        (
            HideRefusal::Failed {
                cause: "holder_write",
                detail: "denied".into(),
            },
            ErrorCode::ActionFailed,
            "cursor_hide_failed",
        ),
        (
            HideRefusal::Incompatible { pid: 7 },
            ErrorCode::ActionFailed,
            "cursor_renderer_incompatible",
        ),
        (
            HideRefusal::UnknownOverlay {
                pid: Some(9),
                output: "DP-1".into(),
            },
            ErrorCode::ActionFailed,
            "cursor_unknown_overlay",
        ),
    ];
    for (refusal, code, kind) in cases {
        let error = refusal.to_error();
        assert_eq!(error.code, code, "{kind}");
        assert_eq!(error.disposition, DeliverySemantics::not_delivered());
        let details = error.details.expect("details");
        assert_eq!(details["kind"], kind);
        assert_eq!(details["physical_delivery_started"], false);
    }
}
