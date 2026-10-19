use std::time::{Duration, Instant};

use agent_desktop_core::{DeliveryDisposition, ErrorCode};

use super::run_with;
use crate::input::keymap_edit::KeymapEdit;
use crate::input::plan::Plan;
use crate::input::step::Step;
use crate::input::x11::fake_sink::{
    FakeSink, Recorded, deadline, drag_plan, events, no_gate, open, plan, start_keymap,
};

fn wait_for(fake: &FakeSink, done: impl Fn(&[Recorded]) -> bool) {
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(5) {
        if done(&fake.recorded()) {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("the worker never reached the expected state");
}

fn remap_plan() -> Plan {
    plan(
        vec![
            Step::Keymap(KeymapEdit {
                keycode: 12,
                keysyms: vec![0x1234, 0x1234],
            }),
            Step::Key {
                code: 12,
                down: true,
            },
            Step::Key {
                code: 12,
                down: false,
            },
        ],
        None,
    )
}

#[test]
fn a_clean_gesture_delivers_every_step_and_reports_the_count() {
    let fake = FakeSink::new();

    let delivery = run_with(
        open(&fake),
        vec![drag_plan()],
        &mut no_gate(),
        deadline(5_000),
    )
    .expect("delivered");

    assert_eq!(delivery.delivered_events, 5);
    assert_eq!(events(&fake).len(), 5);
}

#[test]
fn a_sink_failing_on_the_fourth_event_ends_with_a_release_at_the_origin() {
    let fake = FakeSink::new().failing_on(4);

    let error = run_with(
        open(&fake),
        vec![drag_plan()],
        &mut no_gate(),
        deadline(5_000),
    )
    .expect_err("aborted");

    let recorded = events(&fake);
    assert_eq!(
        recorded[recorded.len() - 2..],
        [Recorded::Move(10, 10), Recorded::Button(1, false)]
    );
    let details = error.details.expect("evidence");
    assert_eq!(details["delivered_events"], 3);
    assert_eq!(details["emergency_release_posted"], true);
    assert_eq!(
        error.disposition.delivery(),
        DeliveryDisposition::DeliveredUnverified
    );
}

#[test]
fn a_deadline_inside_a_dwell_releases_and_reports_a_timeout_with_evidence() {
    let fake = FakeSink::new();
    let dwell = plan(
        vec![
            Step::MoveTo { x: 10, y: 10 },
            Step::Button {
                code: 1,
                down: true,
            },
            Step::Pause(Duration::from_secs(10)),
            Step::Button {
                code: 1,
                down: false,
            },
        ],
        Some((10, 10)),
    );

    let error =
        run_with(open(&fake), vec![dwell], &mut no_gate(), deadline(150)).expect_err("timed out");

    assert_eq!(error.code, ErrorCode::Timeout);
    let details = error.details.expect("evidence");
    assert_eq!(details["delivered_events"], 2);
    assert_eq!(details["physical_delivery_started"], true);
    assert_eq!(events(&fake).last(), Some(&Recorded::Button(1, false)));
}

#[test]
fn a_caller_whose_deadline_passed_gets_the_timeout_and_the_worker_still_releases() {
    let fake = FakeSink::new();
    fake.world().move_stall = Duration::from_millis(900);
    let gesture = plan(
        vec![
            Step::Button {
                code: 1,
                down: true,
            },
            Step::MoveTo { x: 40, y: 40 },
            Step::MoveTo { x: 50, y: 50 },
            Step::Button {
                code: 1,
                down: false,
            },
        ],
        Some((5, 5)),
    );
    let started = Instant::now();

    let error =
        run_with(open(&fake), vec![gesture], &mut no_gate(), deadline(100)).expect_err("timed out");

    assert!(started.elapsed() < Duration::from_millis(800));
    assert_eq!(error.code, ErrorCode::Timeout);
    assert_eq!(error.details.expect("evidence")["delivered_events"], 1);
    wait_for(&fake, |recorded| {
        recorded.contains(&Recorded::Button(1, false))
    });
    assert!(!events(&fake).contains(&Recorded::Move(50, 50)));
}

#[test]
fn a_worker_that_panics_after_a_press_still_releases() {
    let fake = FakeSink::new().panicking_on(2);
    let gesture = plan(
        vec![
            Step::Button {
                code: 1,
                down: true,
            },
            Step::MoveTo { x: 9, y: 9 },
            Step::Button {
                code: 1,
                down: false,
            },
        ],
        None,
    );

    let error = run_with(open(&fake), vec![gesture], &mut no_gate(), deadline(5_000))
        .expect_err("worker lost");

    assert_eq!(events(&fake).last(), Some(&Recorded::Button(1, false)));
    assert_eq!(error.details.expect("evidence")["delivered_events"], 1);
}

#[test]
fn a_remap_is_restored_after_success() {
    let fake = FakeSink::new();

    run_with(
        open(&fake),
        vec![remap_plan()],
        &mut no_gate(),
        deadline(5_000),
    )
    .expect("typed");

    assert_eq!(fake.world().keymap, start_keymap());
}

#[test]
fn a_remap_is_restored_after_a_failure_between_the_remap_and_the_press() {
    let fake = FakeSink::new().failing_on(2);

    let error = run_with(
        open(&fake),
        vec![remap_plan()],
        &mut no_gate(),
        deadline(5_000),
    )
    .expect_err("failed");

    assert_eq!(error.code, ErrorCode::ActionFailed);
    assert_eq!(fake.world().keymap, start_keymap());
}

#[test]
fn a_remap_is_restored_after_the_deadline_cancels_the_call() {
    let fake = FakeSink::new();
    let slow = plan(
        vec![
            remap_plan().steps.remove(0),
            Step::Pause(Duration::from_secs(10)),
        ],
        None,
    );

    let _ = run_with(open(&fake), vec![slow], &mut no_gate(), deadline(200));

    assert_eq!(fake.world().keymap, start_keymap());
}

fn tap_plan(code: u8) -> Plan {
    plan(
        vec![
            Step::Key { code, down: true },
            Step::Key { code, down: false },
        ],
        None,
    )
}

#[test]
fn a_failure_in_a_later_plan_is_delivered_unverified_with_the_whole_count() {
    let fake = FakeSink::new().failing_on(3);

    let error = run_with(
        open(&fake),
        vec![tap_plan(38), tap_plan(39)],
        &mut no_gate(),
        deadline(5_000),
    )
    .expect_err("the second plan fails");

    assert_eq!(
        error.disposition.delivery(),
        DeliveryDisposition::DeliveredUnverified
    );
    assert_eq!(error.details.expect("evidence")["delivered_events"], 2);
}

#[test]
fn a_gate_that_stops_a_later_plan_is_delivered_unverified() {
    let fake = FakeSink::new();
    let mut calls = 0;
    let mut gate = || {
        calls += 1;
        if calls == 2 {
            Err(agent_desktop_core::AdapterError::internal("window lost"))
        } else {
            Ok(())
        }
    };

    let error = run_with(
        open(&fake),
        vec![tap_plan(38), tap_plan(39)],
        &mut gate,
        deadline(5_000),
    )
    .expect_err("the gate stops the second plan");

    assert_eq!(
        error.disposition.delivery(),
        DeliveryDisposition::DeliveredUnverified
    );
    assert_eq!(error.details.expect("evidence")["delivered_events"], 2);
}
