use crate::{
    Action, ActionOps, ActionRequest, ActionResult, AdapterError, Deadline, DeliverySemantics,
    Direction, ElementState, ErrorCode, InputOps, InteractionLease, LiveElement, NativeHandle,
    ObservationOps, RefEntry, SystemOps, WindowInfo,
};
use serde_json::json;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Default)]
struct RefusingAdapter {
    asked: Mutex<Vec<String>>,
    resolutions: AtomicUsize,
    focused: AtomicUsize,
    dispatched: AtomicUsize,
}

fn unproven() -> AdapterError {
    AdapterError::new(ErrorCode::ActionFailed, "the receiver is unproven")
        .with_details(json!({ "cause": "receiver_unproven", "physical_delivery_started": false }))
        .with_disposition(DeliverySemantics::not_delivered())
}

impl ObservationOps for RefusingAdapter {
    fn resolve_element_strict(
        &self,
        _: &RefEntry,
        _: Deadline,
    ) -> Result<NativeHandle, AdapterError> {
        self.resolutions.fetch_add(1, Ordering::SeqCst);
        Ok(NativeHandle::null())
    }

    fn get_live_element(&self, _: &NativeHandle, _: Deadline) -> Result<LiveElement, AdapterError> {
        Ok(LiveElement {
            identity: crate::adapter::live_identity("Run"),
            state: ElementState {
                role: "button".into(),
                states: vec![],
                value: None,
                enabled: Some(true),
                hidden: Some(false),
                offscreen: Some(false),
            },
            states_complete: true,
            bounds: Some(bounds()),
            available_actions: vec![crate::actions::capability::CLICK.into()],
        })
    }

    fn get_element_bounds(
        &self,
        _: &NativeHandle,
        _: Deadline,
    ) -> Result<Option<crate::Rect>, AdapterError> {
        Ok(Some(bounds()))
    }
    crate::adapter::saved_ref_scope!("button", Some("Run"));
}

impl ActionOps for RefusingAdapter {
    fn execute_action(
        &self,
        _: &NativeHandle,
        _: ActionRequest,
        _: &InteractionLease,
    ) -> Result<ActionResult, AdapterError> {
        self.dispatched.fetch_add(1, Ordering::SeqCst);
        Ok(ActionResult::delivered_unverified("click"))
    }
}

impl InputOps for RefusingAdapter {
    fn ref_pointer_refusal(
        &self,
        _: &RefEntry,
        action: &Action,
        _: Deadline,
    ) -> Option<AdapterError> {
        self.asked.lock().unwrap().push(action.name().into());
        Some(unproven())
    }
}

impl SystemOps for RefusingAdapter {
    crate::adapter::guarded_interaction_lease!();

    fn resolve_window_strict(
        &self,
        window: &WindowInfo,
        _: Deadline,
    ) -> Result<WindowInfo, AdapterError> {
        Ok(window.clone())
    }

    fn focus_window(&self, _: &WindowInfo, _: &InteractionLease) -> Result<(), AdapterError> {
        self.focused.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

fn bounds() -> crate::Rect {
    crate::Rect {
        x: 1.0,
        y: 1.0,
        width: 20.0,
        height: 20.0,
    }
}

fn entry() -> RefEntry {
    serde_json::from_value(json!({
        "pid": 1, "process_instance": "test-instance", "role": "button", "name": "Run",
        "source_window_id": "w-1", "source_surface": "window", "path": [0], "unique_at_capture": true,
        "bounds": bounds(), "bounds_hash": bounds().bounds_hash(), "available_actions": ["Click"], "states": []
    }))
    .unwrap()
}

fn pointer_actions() -> Vec<Action> {
    vec![
        Action::Click,
        Action::DoubleClick,
        Action::TripleClick,
        Action::RightClick,
        Action::Scroll(Direction::Down, 3),
    ]
}

#[test]
fn a_headed_ref_pointer_action_the_adapter_refuses_never_activates_resolves_or_sends() {
    for timeout_ms in [None, Some(2_000)] {
        for action in pointer_actions() {
            let _clock = crate::wait::deadline::VirtualClock::freeze();
            let adapter = RefusingAdapter::default();
            let request = ActionRequest::headed(action.clone()).with_timeout_ms(timeout_ms);

            let error = crate::ref_action::execute_entry(&adapter, &entry(), request)
                .expect_err("the adapter refuses the receiver");

            assert_eq!(error.code, ErrorCode::ActionFailed, "{}", action.name());
            assert_eq!(
                error.details.as_ref().unwrap()["cause"],
                "receiver_unproven"
            );
            assert_eq!(
                serde_json::to_value(error.disposition).unwrap()["delivery"],
                "not_delivered"
            );
            assert_eq!(*adapter.asked.lock().unwrap(), vec![action.name()]);
            assert_eq!(
                adapter.focused.load(Ordering::SeqCst),
                0,
                "{}",
                action.name()
            );
            assert_eq!(
                adapter.resolutions.load(Ordering::SeqCst),
                0,
                "{}",
                action.name()
            );
            assert_eq!(
                adapter.dispatched.load(Ordering::SeqCst),
                0,
                "{}",
                action.name()
            );
        }
    }
}

#[test]
fn a_headless_ref_click_never_asks_for_the_pointer_refusal() {
    let _clock = crate::wait::deadline::VirtualClock::freeze();
    let adapter = RefusingAdapter::default();

    crate::ref_action::execute_entry(&adapter, &entry(), ActionRequest::headless(Action::Click))
        .expect("a semantic click has no pointer receiver to prove");

    assert!(adapter.asked.lock().unwrap().is_empty());
    assert_eq!(adapter.dispatched.load(Ordering::SeqCst), 1);
}

#[test]
fn a_headed_keyboard_ref_action_never_asks_for_the_pointer_refusal() {
    let _clock = crate::wait::deadline::VirtualClock::freeze();
    let adapter = RefusingAdapter::default();

    crate::ref_action::execute_entry(
        &adapter,
        &entry(),
        ActionRequest::headed(Action::TypeText("hi".into())),
    )
    .ok();

    assert!(adapter.asked.lock().unwrap().is_empty());
}
