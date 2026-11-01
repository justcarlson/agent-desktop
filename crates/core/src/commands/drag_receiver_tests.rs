use super::*;

struct EndpointProofAdapter {
    refused_pid: Option<u32>,
    proved_pids: Mutex<Vec<u32>>,
    captured: Mutex<Option<DragParams>>,
}

impl EndpointProofAdapter {
    fn refusing(refused_pid: Option<u32>) -> Self {
        Self {
            refused_pid,
            proved_pids: Mutex::new(Vec::new()),
            captured: Mutex::new(None),
        }
    }
}

impl ObservationOps for EndpointProofAdapter {
    fn resolve_element_strict(
        &self,
        _entry: &RefEntry,
        _deadline: crate::Deadline,
    ) -> Result<NativeHandle, AdapterError> {
        Ok(NativeHandle::null())
    }

    fn get_element_bounds(
        &self,
        _handle: &NativeHandle,
        _deadline: crate::Deadline,
    ) -> Result<Option<Rect>, AdapterError> {
        Ok(Some(Rect {
            x: 10.0,
            y: 20.0,
            width: 40.0,
            height: 60.0,
        }))
    }

    fn hit_test(
        &self,
        _handle: &NativeHandle,
        _point: crate::Point,
        _deadline: crate::Deadline,
    ) -> Result<HitTestResult, AdapterError> {
        Ok(HitTestResult::Unknown)
    }

    fn confirm_drag_receiver(
        &self,
        entry: &RefEntry,
        _point: crate::Point,
        _deadline: crate::Deadline,
    ) -> Result<(), AdapterError> {
        let pid = entry.process.pid.get();
        self.proved_pids.lock().unwrap().push(pid);
        if self.refused_pid == Some(pid) {
            return Err(AdapterError::new(
                crate::ErrorCode::ActionFailed,
                "the target window is not shown on any output",
            )
            .with_disposition(DeliverySemantics::not_delivered()));
        }
        Ok(())
    }
}

impl ActionOps for EndpointProofAdapter {}

impl InputOps for EndpointProofAdapter {
    fn drag(
        &self,
        params: DragParams,
        _lease: &crate::InteractionLease,
    ) -> Result<crate::OperationReport, AdapterError> {
        *self.captured.lock().unwrap() = Some(params);
        Ok(crate::adapter::test_report())
    }
}

impl SystemOps for EndpointProofAdapter {
    crate::adapter::guarded_interaction_lease!();

    fn resolve_window_strict(
        &self,
        window: &crate::WindowInfo,
        _deadline: crate::Deadline,
    ) -> Result<crate::WindowInfo, AdapterError> {
        Ok(window.clone())
    }

    fn focus_window(
        &self,
        _window: &crate::WindowInfo,
        _lease: &crate::InteractionLease,
    ) -> Result<(), AdapterError> {
        Ok(())
    }
}

fn drag_by_ref(adapter: &EndpointProofAdapter) -> Result<Value, AppError> {
    let snapshot_id = cross_app_snapshot();
    execute(
        cross_app_args(snapshot_id),
        adapter,
        &CommandContext::default().with_headed(true),
    )
}

#[test]
fn a_ref_drag_whose_destination_is_refused_sends_nothing() {
    let _guard = HomeGuard::new();
    let adapter = EndpointProofAdapter::refusing(Some(2));

    let error = drag_by_ref(&adapter).expect_err("the destination has no receiver proof");

    assert_eq!(error.code(), "ACTION_FAILED");
    assert!(adapter.captured.lock().unwrap().is_none());
}

#[test]
fn a_ref_drag_whose_pickup_is_refused_sends_nothing() {
    let _guard = HomeGuard::new();
    let adapter = EndpointProofAdapter::refusing(Some(1));

    drag_by_ref(&adapter).expect_err("the pickup has no receiver proof");

    assert!(adapter.captured.lock().unwrap().is_none());
}

#[test]
fn a_ref_drag_proves_both_endpoints_before_it_moves() {
    let _guard = HomeGuard::new();
    let adapter = EndpointProofAdapter::refusing(None);

    drag_by_ref(&adapter).expect("both endpoints are proved");

    assert_eq!(*adapter.proved_pids.lock().unwrap(), vec![1, 2]);
    assert!(adapter.captured.lock().unwrap().is_some());
}

#[test]
fn a_coordinate_drag_asks_for_no_receiver_proof() {
    let adapter = EndpointProofAdapter::refusing(Some(1));

    execute(
        xy_args(None),
        &adapter,
        &CommandContext::default().with_headed(true),
    )
    .expect("a coordinate drag has no ref to prove");

    assert!(adapter.proved_pids.lock().unwrap().is_empty());
    assert!(adapter.captured.lock().unwrap().is_some());
}
