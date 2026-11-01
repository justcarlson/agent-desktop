use serde_json::json;

use super::*;
use crate::adapter::{ActionOps, InputOps, ObservationOps, SystemOps};
use crate::{CommandContext, DeliverySemantics, InteractionLease};

/// An activation the window manager refused after it was sent, the way the
/// bound Hyprland reports a covered window: no input event was sent yet.
struct RefusedActivation {
    started: bool,
}

impl ObservationOps for RefusedActivation {}
impl ActionOps for RefusedActivation {}
impl InputOps for RefusedActivation {}

impl SystemOps for RefusedActivation {
    crate::adapter::guarded_interaction_lease!();

    fn resolve_window_strict(
        &self,
        window: &WindowInfo,
        _deadline: crate::Deadline,
    ) -> Result<WindowInfo, AdapterError> {
        Ok(window.clone())
    }

    fn focus_window(&self, _: &WindowInfo, _: &InteractionLease) -> Result<(), AdapterError> {
        Err(AdapterError::new(
            ErrorCode::ActionFailed,
            "Hyprland left another window active",
        )
        .with_details(json!({
            "cause": "activation_refused",
            "activation_sent": true,
            "physical_delivery_started": self.started,
            "side_effects": [{"kind": "cursor_moved"}],
        }))
        .with_disposition(DeliverySemantics::delivered_unverified()))
    }
}

fn window() -> WindowInfo {
    WindowInfo {
        id: "w-1".into(),
        title: "Pad".into(),
        app: "pad".into(),
        pid: crate::ProcessId::new(7),
        process_instance: Some("boot:1".into()),
        bounds: None,
        state: crate::WindowState::default(),
    }
}

fn refusal(adapter: &RefusedActivation) -> AdapterError {
    let deadline = crate::Deadline::after(2_000).expect("deadline");
    let lease = adapter.acquire_interaction_lease(deadline).expect("lease");
    match focus_exact_window(&window(), adapter, &CommandContext::default(), &lease) {
        Err(AppError::Adapter(error)) => error,
        other => panic!("expected an adapter refusal, got {other:?}"),
    }
}

#[test]
fn an_activation_refused_before_any_input_reports_the_input_not_delivered() {
    let error = refusal(&RefusedActivation { started: false });
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
    let details = error.details.expect("details");
    assert_eq!(details["cause"], "activation_refused");
    assert_eq!(details["activation_sent"], true);
    assert_eq!(details["side_effects"][0]["kind"], "cursor_moved");
}

#[test]
fn a_refusal_that_does_not_say_input_never_started_keeps_its_disposition() {
    let error = refusal(&RefusedActivation { started: true });
    assert_eq!(error.disposition, DeliverySemantics::delivered_unverified());
}
