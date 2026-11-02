use super::*;

pub(super) struct StaleThenOkAdapter {
    pub(super) retry: StaleRetryCounter,
    minimum_resolves_before_lease: u32,
    lease_acquisitions: AtomicU32,
}

impl StaleThenOkAdapter {
    pub(super) fn new(fail_until: u32) -> Self {
        Self {
            retry: StaleRetryCounter::new(fail_until),
            minimum_resolves_before_lease: 0,
            lease_acquisitions: AtomicU32::new(0),
        }
    }

    fn expect_poll_before_lease(mut self, minimum: u32) -> Self {
        self.minimum_resolves_before_lease = minimum;
        self
    }
}

impl ObservationOps for StaleThenOkAdapter {
    fn resolve_element_strict(
        &self,
        _entry: &RefEntry,
        _deadline: crate::Deadline,
    ) -> Result<NativeHandle, AdapterError> {
        self.retry.attempt()
    }

    fn get_element_bounds(
        &self,
        _handle: &NativeHandle,
        _deadline: crate::Deadline,
    ) -> Result<Option<Rect>, AdapterError> {
        Ok(Some(Rect {
            x: 100.0,
            y: 200.0,
            width: 20.0,
            height: 10.0,
        }))
    }

    fn hit_test(
        &self,
        _handle: &NativeHandle,
        _point: crate::Point,
        _deadline: crate::Deadline,
    ) -> Result<HitTestResult, AdapterError> {
        Ok(HitTestResult::ReachesTarget)
    }
    crate::adapter::saved_ref_scope!("button", Some("Target"));
}

impl ActionOps for StaleThenOkAdapter {}

impl InputOps for StaleThenOkAdapter {
    fn mouse_event(
        &self,
        _event: MouseEvent,
        _lease: &crate::InteractionLease,
    ) -> Result<crate::OperationReport, AdapterError> {
        Ok(crate::adapter::test_report())
    }
}

impl SystemOps for StaleThenOkAdapter {
    fn acquire_interaction_lease(
        &self,
        deadline: crate::Deadline,
    ) -> Result<crate::InteractionLease, AdapterError> {
        assert!(self.retry.calls() >= self.minimum_resolves_before_lease);
        self.lease_acquisitions.fetch_add(1, Ordering::SeqCst);
        crate::InteractionLease::guarded(deadline, ())
    }

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

/// Regression for the F2 fix: `hover --ref` previously had no `--timeout-ms`
/// at all, so a transient `STALE_REF` on the resolved ref failed the command
/// outright. This proves the wired budget retries through the real
/// `hover::execute` path.
#[test]
fn transient_stale_ref_retries_then_succeeds_when_timeout_wired() {
    let _guard = HomeGuard::new();
    let snapshot_id = ref_snapshot(42);
    let adapter = StaleThenOkAdapter::new(2).expect_poll_before_lease(3);

    let value = execute(
        HoverArgs {
            ref_id: Some(format!("@{}:e1", snapshot_id)),
            xy: None,
            duration_ms: None,
            timeout_ms: Some(5_000),
        },
        &adapter,
        &CommandContext::default().with_headed(true),
    )
    .unwrap();

    assert_eq!(value["disposition"]["delivery"], "delivered_unverified");
    assert!(value.get("hovered").is_none());
    assert!(adapter.retry.calls() >= 3);
    assert_eq!(adapter.lease_acquisitions.load(Ordering::SeqCst), 1);
}
