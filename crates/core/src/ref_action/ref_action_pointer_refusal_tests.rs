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
    observations: AtomicUsize,
    context_checks: AtomicUsize,
    input_events: AtomicUsize,
    receiver_proven: bool,
    broken_identity: Option<BrokenIdentity>,
}

#[derive(Clone, Copy, Debug)]
enum BrokenIdentity {
    NotUniqueAtCapture,
    WindowGone,
    LiveTwins,
    PopupContextRejected,
}

impl RefusingAdapter {
    fn breaking(broken: BrokenIdentity) -> Self {
        Self {
            broken_identity: Some(broken),
            ..Self::default()
        }
    }

    fn proving(self) -> Self {
        Self {
            receiver_proven: true,
            ..self
        }
    }

    fn touched_the_app(&self) -> usize {
        [
            &self.resolutions,
            &self.focused,
            &self.dispatched,
            &self.observations,
            &self.context_checks,
            &self.input_events,
        ]
        .iter()
        .map(|counter| counter.load(Ordering::SeqCst))
        .sum()
    }
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
    fn observe_tree(
        &self,
        root: crate::locator::live_locator::ObservationRoot<'_>,
        _: &crate::locator::live_locator::ObservationRequest,
    ) -> Result<crate::locator::live_locator::ObservedTree, AdapterError> {
        self.observations.fetch_add(1, Ordering::SeqCst);
        let run = ("button", Some("Run"), None);
        match self.broken_identity {
            Some(BrokenIdentity::LiveTwins) => {
                crate::adapter::saved_ref_tree_nodes(root, &[run, run])
            }
            _ => crate::adapter::saved_ref_tree(root, "button", Some("Run")),
        }
    }

    fn validate_saved_ref_context(
        &self,
        _: &RefEntry,
        _: Option<&str>,
    ) -> Result<(), AdapterError> {
        self.context_checks.fetch_add(1, Ordering::SeqCst);
        match self.broken_identity {
            Some(BrokenIdentity::PopupContextRejected) => Err(AdapterError::new(
                ErrorCode::StaleRef,
                "the popup that held this ref is no longer bound to its parent",
            )),
            _ => Ok(()),
        }
    }

    crate::adapter::saved_ref_scope!(@inventory);
    crate::adapter::saved_ref_anchor!();
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
        (!self.receiver_proven).then(unproven)
    }

    fn mouse_event(
        &self,
        _: crate::MouseEvent,
        _: &InteractionLease,
    ) -> Result<crate::OperationReport, AdapterError> {
        self.input_events.fetch_add(1, Ordering::SeqCst);
        Ok(crate::adapter::test_report())
    }

    fn key_event(
        &self,
        _: &crate::KeyCombo,
        _: bool,
        _: &InteractionLease,
    ) -> Result<(), AdapterError> {
        self.input_events.fetch_add(1, Ordering::SeqCst);
        Ok(())
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

fn broken_entry(broken: BrokenIdentity) -> RefEntry {
    let mut entry = entry();
    match broken {
        BrokenIdentity::NotUniqueAtCapture => entry.identity.unique_at_capture = false,
        BrokenIdentity::WindowGone => entry.source.source_window_id = Some("w-gone".into()),
        BrokenIdentity::LiveTwins | BrokenIdentity::PopupContextRejected => {}
    }
    entry
}

const BROKEN_IDENTITIES: [BrokenIdentity; 4] = [
    BrokenIdentity::NotUniqueAtCapture,
    BrokenIdentity::WindowGone,
    BrokenIdentity::LiveTwins,
    BrokenIdentity::PopupContextRejected,
];

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

#[test]
fn a_ref_that_cannot_be_re_identified_still_meets_the_receiver_refusal_first() {
    for broken in BROKEN_IDENTITIES {
        for timeout_ms in [None, Some(2_000)] {
            for action in pointer_actions() {
                let _clock = crate::wait::deadline::VirtualClock::freeze();
                let adapter = RefusingAdapter::breaking(broken);
                let request = ActionRequest::headed(action.clone()).with_timeout_ms(timeout_ms);

                let error =
                    crate::ref_action::execute_entry(&adapter, &broken_entry(broken), request)
                        .expect_err("the receiver is refused");

                let case = format!("{broken:?} {} {timeout_ms:?}", action.name());
                assert_eq!(
                    error.details.as_ref().unwrap()["cause"],
                    "receiver_unproven",
                    "{case}"
                );
                assert_eq!(
                    error.disposition,
                    DeliverySemantics::not_delivered(),
                    "{case}"
                );
                assert_eq!(adapter.touched_the_app(), 0, "{case}");
            }
        }
    }
}

#[test]
fn each_broken_identity_fails_resolution_once_the_receiver_is_proven() {
    for broken in BROKEN_IDENTITIES {
        let _clock = crate::wait::deadline::VirtualClock::freeze();
        let adapter = RefusingAdapter::breaking(broken).proving();
        let request = ActionRequest::headed(Action::Click);

        let error = crate::ref_action::execute_entry(&adapter, &broken_entry(broken), request)
            .expect_err("the ref cannot be re-identified");

        let expected = match broken {
            BrokenIdentity::NotUniqueAtCapture | BrokenIdentity::LiveTwins => {
                ErrorCode::AmbiguousTarget
            }
            BrokenIdentity::WindowGone => ErrorCode::WindowNotFound,
            BrokenIdentity::PopupContextRejected => ErrorCode::StaleRef,
        };
        assert_eq!(error.code, expected, "{broken:?}: {error:?}");
        assert_eq!(adapter.asked.lock().unwrap().len(), 1, "{broken:?}");
        assert_eq!(adapter.dispatched.load(Ordering::SeqCst), 0, "{broken:?}");
        assert_eq!(adapter.input_events.load(Ordering::SeqCst), 0, "{broken:?}");
    }
}
