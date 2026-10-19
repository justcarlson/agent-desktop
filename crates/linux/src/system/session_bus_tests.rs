use std::cell::Cell;

use super::*;

fn live() -> Deadline {
    Deadline::after(5_000).expect("a five second deadline builds")
}

#[test]
fn an_expired_deadline_times_out_before_the_opener_runs() {
    let slot: Mutex<Option<u32>> = Mutex::new(None);
    let opened = Cell::new(0);
    let expired = Deadline::after(0).expect("a zero deadline builds");
    let error = cached_or_open(&slot, expired, || {
        opened.set(opened.get() + 1);
        Ok(1)
    })
    .expect_err("an expired deadline must not open");
    assert_eq!(error.code, ErrorCode::Timeout);
    assert_eq!(opened.get(), 0);
}

#[test]
fn a_failed_open_is_returned_and_the_next_call_opens_again() {
    let slot: Mutex<Option<u32>> = Mutex::new(None);
    let failure = cached_or_open(&slot, live(), || Err(AdapterError::internal("no bus")))
        .expect_err("the open failure is returned");
    assert_eq!(failure.code, ErrorCode::Internal);
    let retried = cached_or_open(&slot, live(), || Ok(7)).expect("the next call opens again");
    assert_eq!(retried, 7);
}

#[test]
fn a_kept_connection_is_reused_without_opening_again() {
    let slot: Mutex<Option<u32>> = Mutex::new(None);
    cached_or_open(&slot, live(), || Ok(3)).expect("the first call opens");
    let opened = Cell::new(false);
    let second = cached_or_open(&slot, live(), || {
        opened.set(true);
        Ok(9)
    })
    .expect("the second call reuses");
    assert_eq!(second, 3);
    assert!(!opened.get());
}

#[test]
fn a_missing_session_bus_names_the_session_bus_and_not_accessibility() {
    let refused = zbus::Error::InputOutput(std::sync::Arc::new(std::io::Error::from(
        ErrorKind::NotFound,
    )));
    let error = unreachable(&refused);
    assert_eq!(error.code, ErrorCode::PlatformNotSupported);
    assert!(error.message.contains("session bus"));
    assert!(!error.message.contains("AT-SPI"));
}

#[test]
fn a_session_bus_timeout_is_a_timeout() {
    let slow = zbus::Error::InputOutput(std::sync::Arc::new(std::io::Error::from(
        ErrorKind::TimedOut,
    )));
    assert_eq!(unreachable(&slow).code, ErrorCode::Timeout);
}

#[test]
fn a_forgotten_connection_is_opened_again_and_another_one_is_left_alone() {
    let slot: Mutex<Option<u32>> = Mutex::new(None);
    cached_or_open(&slot, live(), || Ok(3)).expect("the first call opens");

    forget_matching(&slot, |held| *held == 4);
    let kept = cached_or_open(&slot, live(), || Ok(9)).expect("reuse");
    assert_eq!(kept, 3);

    forget_matching(&slot, |held| *held == 3);
    let reopened = cached_or_open(&slot, live(), || Ok(9)).expect("reopen");
    assert_eq!(reopened, 9);
}

#[test]
fn a_transport_error_is_told_apart_from_a_method_error() {
    let broken = zbus::Error::InputOutput(std::sync::Arc::new(std::io::Error::from(
        ErrorKind::BrokenPipe,
    )));
    assert!(is_transport_failure(&broken));
    let refused = zbus::Error::Failure("no".into());
    assert!(!is_transport_failure(&refused));
}
