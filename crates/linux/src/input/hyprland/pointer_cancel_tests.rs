use std::sync::Arc;

use agent_desktop_core::{Deadline, DeliverySemantics};

use super::with_release_on_signal;
use crate::input::x11::worker_slot::WorkerSlot;

fn deadline(timeout_ms: u64) -> Deadline {
    Deadline::after(timeout_ms).unwrap_or_else(|error| panic!("{error:?}"))
}

#[test]
fn a_slot_held_by_an_earlier_release_refuses_before_the_run() {
    let slot = Arc::new(WorkerSlot::default());
    let _held = slot
        .claim(deadline(1_000))
        .unwrap_or_else(|error| panic!("{error:?}"));
    let mut ran = false;
    let error = with_release_on_signal(&slot, deadline(20), |_| {
        ran = true;
        Ok(())
    })
    .err()
    .unwrap_or_else(|| panic!("a held slot must refuse"));

    assert!(!ran);
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
    assert_eq!(
        error.details.unwrap_or_default()["cause"],
        "previous_release_outstanding"
    );
}

#[test]
fn the_run_holds_the_slot_until_it_returns() {
    let slot = Arc::new(WorkerSlot::default());
    let held_during_run = with_release_on_signal(&slot, deadline(1_000), |progress| {
        assert!(!progress.is_cancelled());
        Ok(slot.claim(deadline(10)).is_err())
    })
    .unwrap_or_else(|error| panic!("{error:?}"));

    assert!(held_during_run);
    assert!(slot.claim(deadline(10)).is_ok());
}
