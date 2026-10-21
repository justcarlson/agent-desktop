use std::time::{Duration, Instant};

use agent_desktop_core::{DeliverySemantics, ErrorCode};

use std::sync::Arc;

use crate::input::keymap_edit::KeymapEdit;
use crate::input::plan::Plan;
use crate::input::step::Step;
use crate::input::x11::fake_sink::{
    FakeSink, Recorded, deadline, drag_plan, events, run, start_keymap,
};
use crate::input::x11::worker_slot::WorkerSlot;

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
    let mut plan = tap_plan(12);
    plan.steps.insert(
        0,
        Step::Keymap(KeymapEdit {
            keycode: 12,
            keysyms: vec![0x1234, 0x1234],
        }),
    );
    plan
}

#[test]
fn text_planning_recovers_own_pending_remaps_before_reading_the_keymap() {
    use crate::input::{held_input::HeldInput, x11::held_journal::HeldJournal};
    for recovery_fails in [false, true] {
        let dir = std::env::temp_dir().join(format!(
            "ad-planning-remap-{}-{recovery_fails}",
            std::process::id()
        ));
        let fake = FakeSink::new();
        fake.world().keymap.keysyms[8..10].copy_from_slice(&[0x01004f60; 2]);
        if recovery_fails {
            fake.world().fail_on_event = Some(1);
        }
        let journal = HeldJournal::at(dir.clone(), ":7", std::process::id());
        journal
            .record(
                &HeldInput::default(),
                &[KeymapEdit {
                    keycode: 12,
                    keysyms: vec![0, 0],
                }],
            )
            .unwrap();
        let sink = fake.clone();
        let map = super::read_keymap(
            move || Ok(sink),
            Some(journal),
            Arc::default(),
            deadline(5_000),
            None,
        );
        if recovery_fails {
            let error = map.expect_err("an unrestored mapping cannot authorize planning");
            assert_eq!(error.disposition, DeliverySemantics::not_delivered());
            assert!(dir.join(format!("{}.json", std::process::id())).exists());
        } else {
            let map = map.expect("recovered map");
            assert_eq!(map, start_keymap());
            let plans = crate::input::text_plan::plan_text(&map, "你").unwrap();
            run(&fake, plans, &mut || Ok(()), deadline(5_000), None).unwrap();
            assert_eq!(fake.world().keymap, start_keymap());
            assert!(!dir.join(format!("{}.json", std::process::id())).exists());
        }
        std::fs::remove_dir_all(dir).unwrap();
    }
}

#[test]
fn a_clean_gesture_delivers_every_step_and_reports_the_count() {
    let fake = FakeSink::new();

    let delivery = run(
        &fake,
        vec![drag_plan()],
        &mut || Ok(()),
        deadline(5_000),
        None,
    )
    .expect("delivered");

    assert_eq!(delivery.delivered_events, 5);
    assert_eq!(events(&fake).len(), 5);
}

#[test]
fn a_sink_failing_on_the_fourth_event_ends_with_a_release_at_the_origin() {
    let fake = FakeSink::new().failing_on(4);

    let error = run(
        &fake,
        vec![drag_plan()],
        &mut || Ok(()),
        deadline(5_000),
        None,
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
    assert_eq!(details["emergency_release_acknowledged"], false);
    assert_eq!(error.disposition, DeliverySemantics::delivered_unverified());
}

#[test]
fn a_deadline_inside_a_dwell_releases_and_reports_a_timeout_with_evidence() {
    let fake = FakeSink::new();
    let mut dwell = drag_plan();
    dwell.steps.insert(2, Step::Pause(Duration::from_secs(10)));

    let error =
        run(&fake, vec![dwell], &mut || Ok(()), deadline(150), None).expect_err("timed out");

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
    let gesture = stalled_gesture();
    let started = Instant::now();

    let error =
        run(&fake, vec![gesture], &mut || Ok(()), deadline(100), None).expect_err("timed out");

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
    let slot = Arc::new(WorkerSlot::default());
    let fake = FakeSink::new().panicking_on(2);
    let gesture = stalled_gesture();

    let error = run(
        &fake,
        vec![gesture],
        &mut || Ok(()),
        deadline(5_000),
        Some(Arc::clone(&slot)),
    )
    .expect_err("worker lost");

    assert_eq!(events(&fake).last(), Some(&Recorded::Button(1, false)));
    assert_eq!(error.details.expect("evidence")["delivered_events"], 1);
    run(
        &fake,
        vec![tap_plan(39)],
        &mut || Ok(()),
        deadline(5_000),
        Some(slot),
    )
    .expect("the panicking worker released its slot");
}

#[test]
fn a_remap_is_restored_after_success() {
    let fake = FakeSink::new();

    run(
        &fake,
        vec![remap_plan()],
        &mut || Ok(()),
        deadline(5_000),
        None,
    )
    .expect("typed");

    assert_eq!(fake.world().keymap, start_keymap());
}

#[test]
fn a_remap_is_restored_after_a_failure_between_the_remap_and_the_press() {
    let fake = FakeSink::new().failing_on(2);

    let error = run(
        &fake,
        vec![remap_plan()],
        &mut || Ok(()),
        deadline(5_000),
        None,
    )
    .expect_err("failed");

    assert_eq!(error.code, ErrorCode::ActionFailed);
    assert_eq!(fake.world().keymap, start_keymap());
}

#[test]
fn a_remap_is_restored_after_the_deadline_cancels_the_call() {
    let fake = FakeSink::new();
    let slow = Plan {
        steps: vec![
            remap_plan().steps.remove(0),
            Step::Pause(Duration::from_secs(10)),
        ],
        release_origin: None,
    };

    let _ = run(&fake, vec![slow], &mut || Ok(()), deadline(200), None);

    assert_eq!(fake.world().keymap, start_keymap());
}

fn tap_plan(code: u8) -> Plan {
    Plan {
        steps: vec![
            Step::Key { code, down: true },
            Step::Key { code, down: false },
        ],
        release_origin: None,
    }
}

#[test]
fn a_failure_in_a_later_plan_is_delivered_unverified_with_the_whole_count() {
    let fake = FakeSink::new().failing_on(3);

    let error = run(
        &fake,
        vec![tap_plan(38), tap_plan(39)],
        &mut || Ok(()),
        deadline(5_000),
        None,
    )
    .expect_err("the second plan fails");

    assert_eq!(error.disposition, DeliverySemantics::delivered_unverified());
    assert_eq!(error.details.expect("evidence")["delivered_events"], 2);
    assert_eq!(
        events(&fake),
        [
            Recorded::Key(38, true),
            Recorded::Key(38, false),
            Recorded::Key(39, false)
        ]
    );
}

#[test]
fn a_first_gate_refusal_is_safe_to_retry_and_sends_nothing() {
    let fake = FakeSink::new();
    let mut gate = || Err(agent_desktop_core::AdapterError::internal("window lost"));

    let error = run(&fake, vec![tap_plan(38)], &mut gate, deadline(5_000), None)
        .expect_err("the first plan is refused");

    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
    assert!(events(&fake).is_empty());
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

    let error = run(
        &fake,
        vec![tap_plan(38), tap_plan(39)],
        &mut gate,
        deadline(5_000),
        None,
    )
    .expect_err("the gate stops the second plan");

    assert_eq!(error.disposition, DeliverySemantics::delivered_unverified());
    assert_eq!(error.message, "window lost");
    assert_eq!(error.details.expect("evidence")["delivered_events"], 2);
    assert_eq!(
        events(&fake),
        [Recorded::Key(38, true), Recorded::Key(38, false)]
    );
}

fn stalled_gesture() -> Plan {
    let button = |down| Step::Button { code: 1, down };
    Plan {
        steps: vec![
            button(true),
            Step::MoveTo { x: 40, y: 40 },
            Step::MoveTo { x: 50, y: 50 },
            button(false),
        ],
        release_origin: Some((5, 5)),
    }
}

#[test]
fn a_call_that_starts_while_a_timed_out_worker_releases_is_refused_and_sends_nothing() {
    let slot = Arc::new(WorkerSlot::default());
    let stalled = FakeSink::new();
    stalled.world().move_stall = Duration::from_millis(900);
    run(
        &stalled,
        vec![stalled_gesture()],
        &mut || Ok(()),
        deadline(100),
        Some(Arc::clone(&slot)),
    )
    .expect_err("the first call timed out");
    let second = FakeSink::new();

    let error = run(
        &second,
        vec![tap_plan(38)],
        &mut || Ok(()),
        deadline(150),
        Some(slot),
    )
    .expect_err("the worker has not released yet");

    assert_eq!(
        error.details.expect("evidence")["cause"],
        "previous_release_outstanding"
    );
    assert!(events(&second).is_empty());
}

#[test]
fn a_call_with_time_left_waits_for_the_release_and_then_runs_after_it() {
    let slot = Arc::new(WorkerSlot::default());
    let fake = FakeSink::new();
    fake.world().move_stall = Duration::from_millis(500);
    run(
        &fake,
        vec![stalled_gesture()],
        &mut || Ok(()),
        deadline(100),
        Some(Arc::clone(&slot)),
    )
    .expect_err("the first call timed out");

    run(
        &fake,
        vec![tap_plan(38)],
        &mut || Ok(()),
        deadline(5_000),
        Some(slot),
    )
    .expect("the second call runs once the first worker is done");

    let recorded = events(&fake);
    let release = recorded
        .iter()
        .position(|event| *event == Recorded::Button(1, false))
        .expect("the first worker released");
    let press = recorded
        .iter()
        .position(|event| *event == Recorded::Key(38, true))
        .expect("the second call pressed");
    assert!(release < press);
}
