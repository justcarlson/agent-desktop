use serde_json::json;
use std::sync::Mutex;

use super::*;
use crate::adapter::{ActionOps, InputOps, ObservationOps, SystemOps};
use crate::{CommandContext, DeliverySemantics, InteractionLease};

struct RefWindowAdapter {
    events: Mutex<Vec<&'static str>>,
    mismatch: bool,
}

impl crate::ObservationOps for RefWindowAdapter {}
impl crate::ActionOps for RefWindowAdapter {}
impl crate::InputOps for RefWindowAdapter {}
impl crate::SystemOps for RefWindowAdapter {
    fn resolve_ref_focus_window(
        &self,
        entry: &crate::RefEntry,
        _: crate::Deadline,
    ) -> Result<WindowInfo, AdapterError> {
        self.events.lock().unwrap().push("ref_window");
        assert!(entry.source.source_window_id.is_none());
        let mut evidence = entry.clone();
        evidence.source.source_window_id = Some("exact-live-owner".into());
        crate::saved_ref_focus_window(&evidence)
    }

    fn resolve_window_strict(
        &self,
        expected: &WindowInfo,
        _: crate::Deadline,
    ) -> Result<WindowInfo, AdapterError> {
        self.events.lock().unwrap().push("strict_window");
        assert_eq!(expected.id, "exact-live-owner");
        let mut live = expected.clone();
        if self.mismatch {
            live.process_instance = Some("different-generation".into());
        }
        Ok(live)
    }

    fn focus_window(
        &self,
        window: &WindowInfo,
        _: &crate::InteractionLease,
    ) -> Result<(), AdapterError> {
        self.events.lock().unwrap().push("focus");
        assert_eq!(window.id, "exact-live-owner");
        Ok(())
    }
}

#[test]
fn recovered_ref_window_is_strictly_verified_before_focus_without_rewriting_source() {
    for mismatch in [false, true] {
        let adapter = RefWindowAdapter {
            events: Mutex::new(Vec::new()),
            mismatch,
        };
        let mut entry = crate::adapter::minimal_ref_entry("menuitem", None);
        entry.source.source_surface = crate::SnapshotSurface::Menu;
        let original = entry.clone();
        let lease =
            crate::InteractionLease::guarded(crate::Deadline::standard().unwrap(), ()).unwrap();
        let result =
            focus_entry_window(&entry, &adapter, &crate::CommandContext::default(), &lease);
        assert_eq!(entry, original);
        if mismatch {
            assert_eq!(result.unwrap_err().code(), "STALE_REF");
            assert_eq!(
                *adapter.events.lock().unwrap(),
                ["ref_window", "strict_window"]
            );
        } else {
            assert!(result.unwrap());
            assert_eq!(
                *adapter.events.lock().unwrap(),
                ["ref_window", "strict_window", "focus"]
            );
        }
    }
}

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
