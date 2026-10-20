use super::tests::{SequenceAdapter, app, empty_baseline, input};
use super::*;
use crate::AppInfo;
use crate::adapter::{ActionOps, InputOps, ObservationOps, SystemOps};

#[test]
fn a_non_retryable_unresponsive_seed_error_is_returned_not_polled_past() {
    let adapter = SequenceAdapter::new(vec![empty_baseline()]);
    let seed_error = AdapterError::app_unresponsive("calc").with_details(json!({
        "app": "calc",
        "pid": 42,
        "process_instance": "boot:1",
        "retryable": false,
    }));

    let err = wait_for_event(
        input("window-opened", None),
        &adapter,
        Some(Err(seed_error)),
    )
    .unwrap_err();

    let AppError::Adapter(adapter_err) = err else {
        panic!("expected AppError::Adapter");
    };
    assert_eq!(adapter_err.code, ErrorCode::AppUnresponsive);
    assert_eq!(*adapter.calls.lock().unwrap(), 0);
}

#[test]
fn a_non_retryable_unresponsive_poll_error_ends_the_wait_at_once() {
    struct StoppedTargetAdapter {
        calls: std::sync::Mutex<usize>,
    }
    impl ObservationOps for StoppedTargetAdapter {
        fn list_apps(&self, _deadline: crate::Deadline) -> Result<Vec<AppInfo>, AdapterError> {
            Ok(vec![app("Calc", "boot:1")])
        }
    }
    impl ActionOps for StoppedTargetAdapter {}
    impl InputOps for StoppedTargetAdapter {}
    impl SystemOps for StoppedTargetAdapter {
        fn capture_signal_baseline(
            &self,
            _filter: &SignalFilter,
            _deadline: crate::Deadline,
        ) -> Result<SignalBaseline, AdapterError> {
            *self.calls.lock().unwrap() += 1;
            Err(AdapterError::app_unresponsive("calc").with_details(json!({
                "app": "calc",
                "pid": 42,
                "process_instance": "boot:1",
                "retryable": false,
            })))
        }
    }

    let adapter = StoppedTargetAdapter {
        calls: std::sync::Mutex::new(0),
    };
    let err = wait_for_event(input("window-closed", Some("Calc")), &adapter, None).unwrap_err();

    let AppError::Adapter(adapter_err) = err else {
        panic!("expected AppError::Adapter");
    };
    assert_eq!(adapter_err.code, ErrorCode::AppUnresponsive);
    assert_eq!(*adapter.calls.lock().unwrap(), 1);
}
