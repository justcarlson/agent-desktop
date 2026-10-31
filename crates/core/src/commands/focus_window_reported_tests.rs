use super::*;
use crate::adapter::{ActionOps, InputOps, ObservationOps, SystemOps};
use crate::{
    AdapterError, FocusActivation, FocusOutcome, FocusProof, FocusSideEffect, InteractionPolicy,
    WindowInfo,
};
use std::sync::Mutex;

struct ReportingAdapter {
    windows: Vec<WindowInfo>,
    answer: Result<FocusActivation, AdapterError>,
    policies: Mutex<Vec<InteractionPolicy>>,
    plain_focus_calls: Mutex<u32>,
}

impl ObservationOps for ReportingAdapter {
    fn list_windows(
        &self,
        _filter: &WindowFilter,
        _deadline: crate::Deadline,
    ) -> Result<Vec<WindowInfo>, AdapterError> {
        Ok(self.windows.clone())
    }
}

impl ActionOps for ReportingAdapter {}

impl InputOps for ReportingAdapter {}

impl SystemOps for ReportingAdapter {
    crate::adapter::guarded_interaction_lease!();

    fn focus_window(
        &self,
        _win: &WindowInfo,
        _lease: &crate::InteractionLease,
    ) -> Result<(), AdapterError> {
        *self.plain_focus_calls.lock().unwrap() += 1;
        Ok(())
    }

    fn focus_window_reported(
        &self,
        win: &WindowInfo,
        policy: InteractionPolicy,
        _lease: &crate::InteractionLease,
    ) -> Result<FocusOutcome, AdapterError> {
        self.policies.lock().unwrap().push(policy);
        let mut window = win.clone();
        window.state.is_focused = true;
        self.answer
            .clone()
            .map(|activation| FocusOutcome { window, activation })
    }
}

fn window(id: &str) -> WindowInfo {
    WindowInfo {
        id: id.into(),
        title: "Main".into(),
        app: "foot".into(),
        pid: crate::ProcessId::new(42),
        process_instance: Some("test-instance".into()),
        bounds: None,
        state: crate::WindowState::default(),
    }
}

fn adapter(answer: Result<FocusActivation, AdapterError>) -> ReportingAdapter {
    ReportingAdapter {
        windows: vec![window("w-hypr-1-2-a")],
        answer,
        policies: Mutex::new(Vec::new()),
        plain_focus_calls: Mutex::new(0),
    }
}

fn args(policy: InteractionPolicy) -> FocusWindowArgs {
    FocusWindowArgs {
        window_id: Some("w-hypr-1-2-a".into()),
        app: None,
        title: None,
        policy,
    }
}

#[test]
fn a_reported_focus_carries_its_proof_and_side_effects_and_skips_the_plain_path() {
    let adapter = adapter(Ok(FocusActivation {
        activated: true,
        proof: FocusProof::CompositorOnly,
        side_effects: vec![FocusSideEffect::WorkspaceSwitched {
            monitor: "Virtual-1".into(),
            from: "1".into(),
            to: "2".into(),
        }],
    }));

    let value = execute(args(InteractionPolicy::headed()), &adapter).unwrap();

    assert_eq!(value["focused"]["id"], "w-hypr-1-2-a");
    assert_eq!(value["focused"]["is_focused"], true);
    assert_eq!(value["activation"]["activated"], true);
    assert_eq!(value["activation"]["proof"], "compositor_only");
    assert_eq!(
        value["activation"]["side_effects"][0],
        serde_json::json!({"kind": "workspace_switched", "monitor": "Virtual-1", "from": "1", "to": "2"})
    );
    assert_eq!(*adapter.plain_focus_calls.lock().unwrap(), 0);
}

#[test]
fn the_command_policy_reaches_the_adapter_unchanged() {
    let adapter = adapter(Ok(FocusActivation {
        activated: false,
        proof: FocusProof::CompositorAndAccessibility,
        side_effects: Vec::new(),
    }));

    let value = execute(args(InteractionPolicy::headless()), &adapter).unwrap();

    assert_eq!(
        *adapter.policies.lock().unwrap(),
        vec![InteractionPolicy::headless()]
    );
    assert_eq!(value["activation"]["activated"], false);
    assert!(value["activation"].get("side_effects").is_none());
}

#[test]
fn a_reported_refusal_is_returned_without_trying_the_plain_path() {
    let refusal = AdapterError::policy_denied("headless focus-window does not activate windows");
    let adapter = adapter(Err(refusal));

    let error = execute(args(InteractionPolicy::headless()), &adapter).unwrap_err();

    assert_eq!(error.code(), "POLICY_DENIED");
    assert_eq!(*adapter.plain_focus_calls.lock().unwrap(), 0);
}
