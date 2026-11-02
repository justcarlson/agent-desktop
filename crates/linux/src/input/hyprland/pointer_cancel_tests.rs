use std::sync::Arc;
use std::time::{Duration, Instant};

use agent_desktop_core::{AdapterError, Deadline, DeliverySemantics};

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

#[test]
fn a_signalled_run_ends_by_the_signal_without_waiting_out_the_release() {
    const CHILD: &str = "AD_POINTER_SIGNAL_CHILD";
    if std::env::var_os(CHILD).is_none() {
        const FILTER: &str = "input::hyprland::pointer_cancel::tests::a_signalled_run_ends_by_the_signal_without_waiting_out_the_release";
        let started = Instant::now();
        let output = std::process::Command::new(std::env::current_exe().expect("the test binary"))
            .args(["--exact", FILTER, "--nocapture"])
            .env(CHILD, "1")
            .output()
            .expect("the child runs");
        let elapsed = started.elapsed();
        assert_eq!(
            std::os::unix::process::ExitStatusExt::signal(&output.status),
            Some(libc::SIGTERM),
            "the run returned before SIGTERM's own action: {output:?}"
        );
        assert!(
            elapsed < Duration::from_millis(1_500),
            "the slot was held while waiting for the signal: {elapsed:?}"
        );
        return;
    }
    unsafe { libc::signal(libc::SIGTERM, libc::SIG_DFL) };
    crate::input::x11::signal_release::enable();
    let slot = Arc::new(WorkerSlot::default());
    let outcome: Result<(), _> = with_release_on_signal(&slot, deadline(5_000), |progress| {
        unsafe { libc::kill(libc::getpid(), libc::SIGTERM) };
        let started = Instant::now();
        while !progress.is_cancelled() && started.elapsed() < Duration::from_secs(5) {
            std::thread::sleep(Duration::from_millis(1));
        }
        Err(AdapterError::internal("stopped by the signal"))
    });
    drop(outcome);
    std::process::exit(1);
}
