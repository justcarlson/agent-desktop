use crate::adapter::{ActionOps, InputOps, LiveElement, ObservationOps, SystemOps};
use crate::ref_action::*;
use crate::{
    Action, BoundsSpace, Direction, ElementState, ErrorCode, RefGeometry, actions::capability,
};
use std::sync::atomic::{AtomicU32, Ordering};

/// A scroll pane on a bound Hyprland: the snapshot stores its rectangle in
/// window coordinates, and the live read places it on the global layout,
/// so the two never hash alike. The pane's dispatch refuses headless input
/// the way the Linux pointer route does.
struct PlacedPane {
    actions: Vec<String>,
    dispatches: AtomicU32,
}

fn window_rect() -> crate::Rect {
    crate::Rect {
        x: 10.0,
        y: 40.0,
        width: 300.0,
        height: 200.0,
    }
}

fn global_rect() -> crate::Rect {
    crate::Rect {
        x: 1302.0,
        y: 78.0,
        width: 300.0,
        height: 200.0,
    }
}

fn entry(actions: &[String]) -> RefEntry {
    let mut entry = crate::adapter::minimal_ref_entry("scrollarea", None);
    let bounds = window_rect();
    entry.geometry = RefGeometry {
        bounds: Some(bounds),
        bounds_hash: bounds.bounds_hash(),
        bounds_space: Some(BoundsSpace::Window),
    };
    entry.capabilities.available_actions = actions.to_vec();
    entry
}

impl ObservationOps for PlacedPane {
    fn resolve_element_strict(
        &self,
        _: &RefEntry,
        _: crate::Deadline,
    ) -> Result<NativeHandle, AdapterError> {
        Ok(NativeHandle::null())
    }

    fn get_live_element(
        &self,
        _: &NativeHandle,
        _: crate::Deadline,
    ) -> Result<LiveElement, AdapterError> {
        Ok(LiveElement {
            identity: crate::adapter::live_identity(""),
            state: ElementState {
                role: "scrollarea".into(),
                states: Vec::new(),
                value: None,
                enabled: Some(true),
                hidden: Some(false),
                offscreen: Some(false),
            },
            states_complete: true,
            bounds: Some(global_rect()),
            available_actions: self.actions.clone(),
        })
    }

    fn get_element_bounds(
        &self,
        _: &NativeHandle,
        _: crate::Deadline,
    ) -> Result<Option<crate::Rect>, AdapterError> {
        Ok(Some(global_rect()))
    }
}

impl ActionOps for PlacedPane {
    fn execute_action(
        &self,
        _: &NativeHandle,
        request: ActionRequest,
        _: &crate::InteractionLease,
    ) -> Result<ActionResult, AdapterError> {
        self.dispatches.fetch_add(1, Ordering::SeqCst);
        Err(AdapterError::policy_denied_for_policy(
            "Physical pointer input is disabled by the current interaction policy",
            request.policy,
        ))
    }
}
impl InputOps for PlacedPane {}
impl SystemOps for PlacedPane {
    crate::adapter::guarded_interaction_lease!();
}

fn scroll_down() -> ActionRequest {
    ActionRequest::headless(Action::Scroll(Direction::Down, 2)).with_timeout_ms(Some(2_000))
}

#[test]
fn a_headless_scroll_on_a_window_space_ref_reaches_the_policy_refusal() {
    let actions = vec![capability::SCROLL.to_owned()];
    let adapter = PlacedPane {
        actions: actions.clone(),
        dispatches: AtomicU32::new(0),
    };
    let error = execute_entry(&adapter, &entry(&actions), scroll_down())
        .expect_err("a headless scroll is refused at dispatch");
    assert_eq!(error.code, ErrorCode::PolicyDenied, "{}", error.message);
    assert_eq!(adapter.dispatches.load(Ordering::SeqCst), 1);
    assert_eq!(error.disposition, crate::DeliverySemantics::not_delivered());
}

#[test]
fn a_headless_scroll_on_a_pane_without_scroll_stops_on_the_missing_action() {
    let adapter = PlacedPane {
        actions: Vec::new(),
        dispatches: AtomicU32::new(0),
    };
    let error = execute_entry(&adapter, &entry(&[]), scroll_down())
        .expect_err("a pane without Scroll cannot be scrolled");
    assert_eq!(error.code, ErrorCode::ActionNotSupported);
    assert!(
        error
            .message
            .contains("stable (bounds changed since the previous observation)")
            && error.message.contains("Scroll is not available"),
        "{}",
        error.message
    );
    assert_eq!(adapter.dispatches.load(Ordering::SeqCst), 0);
}
