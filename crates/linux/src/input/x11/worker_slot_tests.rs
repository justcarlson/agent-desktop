use std::sync::Arc;
use std::time::Duration;

use super::WorkerSlot;
use crate::input::x11::fake_sink::deadline;

#[test]
fn a_claim_waits_for_the_guard_and_gets_the_slot_when_it_drops() {
    let slot = Arc::new(WorkerSlot::default());
    let held = slot.claim(deadline(1_000)).expect("the first claim");
    let later = Arc::clone(&slot);
    let waiter = std::thread::spawn(move || later.claim(deadline(5_000)).is_ok());
    std::thread::sleep(Duration::from_millis(100));

    drop(held);

    assert!(waiter.join().expect("the waiter ends"));
}

#[test]
fn a_claim_past_its_deadline_is_refused_and_a_panicking_holder_still_frees_the_slot() {
    let slot = Arc::new(WorkerSlot::default());
    let held = slot.claim(deadline(1_000)).expect("the first claim");
    assert!(slot.claim(deadline(50)).is_err());
    drop(held);

    let crashing = Arc::clone(&slot);
    let outcome = std::thread::spawn(move || {
        let _held = crashing.claim(deadline(1_000)).expect("claim");
        panic!("the worker panicked");
    })
    .join();

    assert!(outcome.is_err());
    assert!(slot.claim(deadline(500)).is_ok());
}
