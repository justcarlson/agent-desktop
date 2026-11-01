use std::time::{Duration, Instant};

use async_io::Timer;

use super::ReadPatience;

const SLICE: Duration = Duration::from_millis(100);
const GRACE: Duration = Duration::from_millis(400);

#[test]
fn a_stopped_process_gets_no_grace_and_a_running_one_keeps_it() {
    assert_eq!(
        ReadPatience::for_state(SLICE, GRACE, true).grace,
        Duration::ZERO
    );
    assert_eq!(ReadPatience::for_state(SLICE, GRACE, false).grace, GRACE);
    assert_eq!(
        ReadPatience::for_process(SLICE, GRACE, std::process::id()).grace,
        GRACE
    );
}

#[test]
fn an_answer_after_the_slice_and_inside_the_grace_is_kept() {
    let patience = ReadPatience::for_state(SLICE, GRACE, false);
    let answer = zbus::block_on(patience.wait(async {
        Timer::after(SLICE * 2).await;
        7
    }));
    assert_eq!(answer, Some(7));
}

#[test]
fn without_grace_an_answer_after_the_slice_is_dropped() {
    let patience = ReadPatience::for_state(SLICE, GRACE, true);
    let started = Instant::now();
    let answer = zbus::block_on(patience.wait(async {
        Timer::after(SLICE * 2).await;
        7
    }));
    assert_eq!(answer, None);
    assert!(started.elapsed() < SLICE * 2);
}
