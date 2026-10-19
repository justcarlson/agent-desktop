use super::*;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

fn signal_builder(member: &'static str) -> zbus::message::Builder<'static> {
    zbus::Message::signal(
        "/org/freedesktop/Notifications",
        "org.freedesktop.Notifications",
        member,
    )
    .expect("a signal builder")
}

#[test]
fn an_action_signal_is_parsed_with_its_id() {
    let message = signal_builder("ActionInvoked")
        .build(&(7u32, "act-b"))
        .expect("built");
    assert_eq!(parse(&message), Some(Signal::ActionInvoked(7)));
}

#[test]
fn a_closed_signal_is_parsed_with_its_id() {
    let message = signal_builder("NotificationClosed")
        .build(&(7u32, 2u32))
        .expect("built");
    assert_eq!(parse(&message), Some(Signal::Closed(7)));
}

#[test]
fn another_member_is_ignored() {
    let message = signal_builder("ActivationToken")
        .build(&(7u32, "t"))
        .expect("built");
    assert_eq!(parse(&message), None);
}

#[test]
fn no_signal_ends_the_wait_at_the_budget() {
    let (_keep, inbox) = channel::<Signal>();
    let started = Instant::now();
    let seen = gather(
        &inbox,
        Duration::from_millis(120),
        Duration::from_millis(30),
    );
    assert!(seen.is_empty());
    assert!(started.elapsed() >= Duration::from_millis(100));
    assert!(started.elapsed() < Duration::from_millis(600));
}

#[test]
fn a_quiet_gap_after_a_signal_ends_the_wait_early() {
    let (outbox, inbox) = channel();
    outbox.send(Signal::ActionInvoked(1)).expect("sent");
    let started = Instant::now();
    let seen = gather(&inbox, Duration::from_secs(5), Duration::from_millis(40));
    assert_eq!(seen, vec![Signal::ActionInvoked(1)]);
    assert!(started.elapsed() < Duration::from_secs(1));
}

#[test]
fn a_signal_after_the_budget_is_not_received() {
    let (outbox, inbox) = channel();
    let seen = gather(&inbox, Duration::from_millis(50), Duration::from_millis(20));
    outbox.send(Signal::ActionInvoked(1)).expect("sent");
    assert!(seen.is_empty());
}

#[test]
fn a_dead_reader_ends_the_wait_without_signals() {
    let (outbox, inbox) = channel::<Signal>();
    drop(outbox);
    let started = Instant::now();
    assert!(gather(&inbox, Duration::from_secs(5), Duration::from_millis(40)).is_empty());
    assert!(started.elapsed() < Duration::from_secs(1));
}

#[test]
fn the_reader_thread_has_ended_when_collect_returns() {
    let ended = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&ended);
    let reader = Reader::spawn(move |stop, outbox| {
        let _ = outbox.send(Signal::Closed(3));
        while !stop.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(5));
        }
        flag.store(true, Ordering::SeqCst);
    });
    let seen = Box::new(reader).collect(Duration::from_millis(300));
    assert_eq!(seen, vec![Signal::Closed(3)]);
    assert!(ended.load(Ordering::SeqCst));
}

#[test]
fn dropping_the_subscription_also_ends_the_thread() {
    let ended = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&ended);
    let reader = Reader::spawn(move |stop, _outbox| {
        while !stop.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(5));
        }
        flag.store(true, Ordering::SeqCst);
    });
    drop(reader);
    assert!(ended.load(Ordering::SeqCst));
}
