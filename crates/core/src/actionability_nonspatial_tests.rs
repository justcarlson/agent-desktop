use super::*;

struct ProtocolTarget(LiveElement);

impl ObservationOps for ProtocolTarget {
    fn is_nonspatial_target(&self, _handle: &NativeHandle) -> bool {
        true
    }

    fn get_live_element(
        &self,
        _handle: &NativeHandle,
        _deadline: crate::Deadline,
    ) -> Result<LiveElement, AdapterError> {
        Ok(self.0.clone())
    }

    fn hit_test(
        &self,
        _handle: &NativeHandle,
        _point: crate::Point,
        _deadline: crate::Deadline,
    ) -> Result<crate::HitTestResult, AdapterError> {
        panic!("a nonspatial target cannot authorize pointer delivery")
    }
}

impl ActionOps for ProtocolTarget {}
impl InputOps for ProtocolTarget {}
impl SystemOps for ProtocolTarget {}

#[test]
fn nonspatial_semantic_targets_need_live_state_but_not_screen_bounds() {
    let (entry, adapter) = target("menuitem", &[capability::CLICK, capability::RIGHT_CLICK]);
    let mut live = adapter.0;
    live.bounds = None;
    let check = |live: LiveElement, request: ActionRequest| {
        check_live(
            &entry,
            &NativeHandle::null(),
            &ProtocolTarget(live),
            &request,
        )
    };
    for action in [Action::Click, Action::RightClick] {
        let report = check(live.clone(), ActionRequest::headless(action.clone()))
            .expect("protocol actions have no screen position");
        assert!(report.actionable);
        assert_eq!(report.pointer_delivery, PointerDelivery::Semantic);
        assert_eq!(report.verified_point, None);
        assert_eq!(report.presentation_point, None);
        for invalid in 0..6 {
            let mut unsafe_live = live.clone();
            match invalid {
                0 => unsafe_live.state.hidden = Some(true),
                1 => unsafe_live.state.offscreen = Some(true),
                2 => unsafe_live.states_complete = false,
                3 => unsafe_live.state.hidden = None,
                4 => unsafe_live.state.enabled = Some(false),
                _ => unsafe_live.available_actions.clear(),
            }
            let error = check(unsafe_live, ActionRequest::headless(action.clone())).unwrap_err();
            assert_eq!(error.disposition, crate::DeliverySemantics::not_delivered());
        }
        assert!(check(live.clone(), ActionRequest::headed(action)).is_err());
    }
    assert!(check(live, ActionRequest::headless(Action::Hover)).is_err());
}
