use super::*;

#[test]
fn the_request_names_source_and_timestamp_in_the_first_two_longs() {
    assert_eq!(activation_data(2, 123_456), [2, 123_456, 0, 0, 0]);
}

#[test]
fn the_message_targets_the_client_with_the_active_window_atom() {
    let event = activation_event(0x0040_0003, 77, ACTIVATION_SOURCE, 5_000);
    assert_eq!(event.window, 0x0040_0003);
    assert_eq!(event.type_, 77);
    assert_eq!(event.format, 32);
    assert_eq!(event.data.as_data32(), [2, 5_000, 0, 0, 0]);
}

#[test]
fn a_preparation_that_outlives_the_deadline_never_leads_to_a_send() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, mpsc::channel};

    let sent = Arc::new(AtomicBool::new(false));
    let (finished, done) = channel();
    let deadline = agent_desktop_core::Deadline::from_duration(Duration::from_millis(50))
        .expect("a deadline builds");
    let flag = Arc::clone(&sent);
    let outcome = request_via(
        deadline,
        move |_| {
            std::thread::sleep(Duration::from_millis(300));
            let _ = finished.send(());
            Ok(())
        },
        move |()| {
            flag.store(true, Ordering::SeqCst);
            Ok(())
        },
    );
    assert_eq!(outcome, Err(ActivateError::SendFailed));
    done.recv_timeout(Duration::from_secs(2))
        .expect("the worker finished");
    assert!(!sent.load(Ordering::SeqCst));
}

#[test]
fn a_preparation_in_time_is_sent_once_on_the_calling_thread() {
    let caller = std::thread::current().id();
    let deadline = agent_desktop_core::Deadline::from_duration(Duration::from_secs(2))
        .expect("a deadline builds");
    let outcome = request_via(
        deadline,
        |_| Ok(7_u32),
        |prepared| {
            assert_eq!(prepared, 7);
            assert_eq!(std::thread::current().id(), caller);
            Ok(())
        },
    );
    assert_eq!(outcome, Ok(()));
}
