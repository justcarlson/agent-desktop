use super::*;

#[test]
fn unproven_pointer_receiver_refuses_once_before_any_move() {
    for timeout_ms in [None, Some(5_000)] {
        let _guard = HomeGuard::new();
        let snapshot_id = ref_snapshot(42);
        let adapter = HoverCaptureAdapter::new().refusing_receiver();

        let error = execute(
            HoverArgs {
                ref_id: Some(format!("@{}:e1", snapshot_id)),
                xy: None,
                duration_ms: None,
                timeout_ms,
            },
            &adapter,
            &CommandContext::default().with_headed(true),
        )
        .unwrap_err();

        let AppError::Adapter(error) = error else {
            panic!("expected an adapter error");
        };
        assert_eq!(error.code, crate::ErrorCode::ActionFailed);
        assert_eq!(error.disposition, crate::DeliverySemantics::not_delivered());
        assert_eq!(adapter.receiver_checks.load(Ordering::SeqCst), 1);
        assert!(adapter.moved_to.lock().unwrap().is_none());
    }
}

#[test]
fn a_ref_hover_the_adapter_refuses_answers_before_any_wait_focus_or_resolution() {
    for timeout_ms in [None, Some(5_000)] {
        let _guard = HomeGuard::new();
        let snapshot_id = ref_snapshot(42);
        let adapter = HoverCaptureAdapter::new().refusing_ref_hover();

        let error = execute(
            HoverArgs {
                ref_id: Some(format!("@{}:e1", snapshot_id)),
                xy: None,
                duration_ms: None,
                timeout_ms,
            },
            &adapter,
            &CommandContext::default().with_headed(true),
        )
        .unwrap_err();

        assert_eq!(error.code(), "ACTION_NOT_SUPPORTED");
        assert_eq!(adapter.resolutions.load(Ordering::SeqCst), 0);
        assert!(adapter.focused_pids.lock().unwrap().is_empty());
        assert_eq!(adapter.receiver_checks.load(Ordering::SeqCst), 0);
        assert!(adapter.moved_to.lock().unwrap().is_none());
    }
}

#[test]
fn a_coordinate_hover_never_asks_for_the_ref_hover_refusal() {
    let _guard = HomeGuard::new();
    let adapter = HoverCaptureAdapter::new().refusing_ref_hover();

    let value = execute(
        HoverArgs {
            ref_id: None,
            xy: Some((40.0, 60.0)),
            duration_ms: None,
            timeout_ms: None,
        },
        &adapter,
        &CommandContext::default().with_headed(true),
    )
    .unwrap();

    let moved = adapter.moved_to.lock().unwrap().clone().unwrap();
    assert!(matches!(moved.kind, MouseEventKind::Move));
    assert_eq!((moved.point.x, moved.point.y), (40.0, 60.0));
    assert_eq!(value["disposition"]["delivery"], "delivered_unverified");
}

#[test]
fn the_pointer_receiver_is_confirmed_on_the_element_core_resolved() {
    let _guard = HomeGuard::new();
    let snapshot_id = ref_snapshot(42);
    let adapter = HoverCaptureAdapter::new();

    execute(
        HoverArgs {
            ref_id: Some(format!("@{}:e1", snapshot_id)),
            xy: None,
            duration_ms: None,
            timeout_ms: None,
        },
        &adapter,
        &CommandContext::default().with_headed(true),
    )
    .expect("the receiver check runs on the resolved handle");

    assert_eq!(adapter.receiver_checks.load(Ordering::SeqCst), 1);
    assert!(adapter.moved_to.lock().unwrap().is_some());
}
