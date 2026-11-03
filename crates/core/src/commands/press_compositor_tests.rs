use super::*;
use crate::adapter::{ActionOps, InputOps, ObservationOps, SystemOps};
use crate::{ActionResult, AdapterError, KeyCombo};
fn window(pid: u32) -> crate::WindowInfo {
    crate::WindowInfo {
        id: format!("w-{pid}"),
        title: "foot".into(),
        app: "foot".into(),
        pid: crate::ProcessId::new(pid),
        process_instance: Some("boot:100".into()),
        bounds: None,
        state: Default::default(),
    }
}

#[test]
fn treeless_window_press_resolves_without_an_accessibility_app_and_reports_proof() {
    struct Treeless;
    impl ObservationOps for Treeless {
        fn list_apps(&self, _: crate::Deadline) -> Result<Vec<crate::AppInfo>, AdapterError> {
            Ok(Vec::new())
        }
        fn list_windows(
            &self,
            _: &crate::WindowFilter,
            _: crate::Deadline,
        ) -> Result<Vec<crate::WindowInfo>, AdapterError> {
            let mut win = window(42);
            win.state.accessible = false;
            Ok(vec![win])
        }
    }
    impl ActionOps for Treeless {}
    impl InputOps for Treeless {}
    impl SystemOps for Treeless {
        crate::adapter::guarded_interaction_lease!();
        fn press_key_for_window(
            &self,
            _: &crate::WindowInfo,
            _: &KeyCombo,
            _: crate::InteractionPolicy,
            _: &crate::InteractionLease,
        ) -> Result<ActionResult, AdapterError> {
            Ok(ActionResult::delivered_unverified("PressKey")
                .with_details(serde_json::json!({"focus_proof": "compositor_only"})))
        }
    }
    let result = execute(
        PressArgs {
            combo: "a".into(),
            app: None,
            window_id: Some("w-42".into()),
            force: false,
        },
        &Treeless,
        &CommandContext::default().with_headed(true),
    )
    .unwrap();
    assert_eq!(result["details"]["focus_proof"], "compositor_only");
}

#[test]
fn treeless_window_does_not_make_a_type_ref_valid() {
    let error = crate::commands::type_text::execute(
        crate::commands::type_text::TypeArgs {
            ref_id: "w-hypr-50-60-a".into(),
            text: "x".into(),
            timeout_ms: None,
            interval_ms: None,
        },
        &crate::adapter::NoopAdapter,
        &CommandContext::default().with_headed(true),
    )
    .unwrap_err();
    assert_eq!(error.code(), "INVALID_ARGS");
}
