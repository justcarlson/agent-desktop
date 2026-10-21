use std::cell::Cell;

use super::*;

fn live() -> Deadline {
    Deadline::after(5_000).expect("a five second deadline builds")
}

#[test]
fn cache_obeys_deadlines_retries_reuses_and_forgets_only_the_matching_connection() {
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

    let failure = cached_or_open(&slot, live(), || Err(AdapterError::internal("no bus")))
        .expect_err("the open failure is returned");
    assert_eq!(failure.code, ErrorCode::Internal);
    let retried = cached_or_open(&slot, live(), || Ok(7)).expect("the next call opens again");
    assert_eq!(retried, 7);
    let reused = cached_or_open(&slot, live(), || {
        opened.set(opened.get() + 1);
        Ok(9)
    })
    .expect("the next call reuses");
    assert_eq!(reused, 7);
    assert_eq!(opened.get(), 0);

    forget_matching(&slot, |held| *held == 4);
    let kept = cached_or_open(&slot, live(), || {
        opened.set(opened.get() + 1);
        Ok(9)
    })
    .expect("a different connection is kept");
    assert_eq!(kept, 7);
    assert_eq!(opened.get(), 0);

    forget_matching(&slot, |held| *held == 7);
    let reopened = cached_or_open(&slot, live(), || {
        opened.set(opened.get() + 1);
        Ok(9)
    })
    .expect("the matching connection is reopened");
    assert_eq!(reopened, 9);
    assert_eq!(opened.get(), 1);
}

#[test]
fn session_bus_errors_keep_their_platform_and_retry_meaning() {
    let refused = zbus::Error::InputOutput(std::sync::Arc::new(std::io::Error::from(
        ErrorKind::NotFound,
    )));
    let error = unreachable(&refused);
    assert_eq!(error.code, ErrorCode::PlatformNotSupported);
    assert!(error.message.contains("session bus"));
    assert!(!error.message.contains("AT-SPI"));

    let slow = zbus::Error::InputOutput(std::sync::Arc::new(std::io::Error::from(
        ErrorKind::TimedOut,
    )));
    assert_eq!(unreachable(&slow).code, ErrorCode::Timeout);

    let broken = zbus::Error::InputOutput(std::sync::Arc::new(std::io::Error::from(
        ErrorKind::BrokenPipe,
    )));
    assert!(is_transport_failure(&broken));
    let refused = zbus::Error::Failure("no".into());
    assert!(!is_transport_failure(&refused));
}
