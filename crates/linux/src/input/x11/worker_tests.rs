use agent_desktop_core::{AdapterError, DeliveryDisposition, ErrorCode};

use crate::input::step::Step;
use crate::input::x11::fake_sink::{
    FakeSink, Recorded, deadline, drag_plan, events, no_gate, open, plan,
};
use crate::input::x11::session::run_with;

#[test]
fn the_gate_runs_before_each_plan_and_a_refusal_stops_the_rest() {
    let fake = FakeSink::new();
    let first = plan(
        vec![
            Step::Key {
                code: 38,
                down: true,
            },
            Step::Key {
                code: 38,
                down: false,
            },
        ],
        None,
    );
    let second = plan(
        vec![
            Step::Key {
                code: 39,
                down: true,
            },
            Step::Key {
                code: 39,
                down: false,
            },
        ],
        None,
    );
    let mut calls = 0;
    let mut gate = || {
        calls += 1;
        if calls == 2 {
            Err(AdapterError::new(ErrorCode::ActionFailed, "focus moved"))
        } else {
            Ok(())
        }
    };

    let error = run_with(open(&fake), vec![first, second], &mut gate, deadline(5_000))
        .expect_err("refused");

    assert_eq!(error.message, "focus moved");
    assert_eq!(error.details.expect("evidence")["delivered_events"], 2);
    assert_eq!(
        events(&fake),
        [Recorded::Key(38, true), Recorded::Key(38, false)]
    );
}

#[test]
fn a_gate_that_refuses_the_first_plan_is_not_delivered() {
    let fake = FakeSink::new();
    let only = plan(
        vec![
            Step::Key {
                code: 38,
                down: true,
            },
            Step::Key {
                code: 38,
                down: false,
            },
        ],
        None,
    );
    let mut gate = || Err(AdapterError::new(ErrorCode::ActionFailed, "not active"));

    let error = run_with(open(&fake), vec![only], &mut gate, deadline(5_000)).expect_err("refused");

    assert_eq!(
        error.disposition.delivery(),
        DeliveryDisposition::NotDelivered
    );
    assert!(events(&fake).is_empty());
}

#[test]
fn a_point_outside_the_root_is_refused_before_any_event() {
    let outside = [
        (i32::MIN, 0),
        (-1, 0),
        (0, -1),
        (1280, 0),
        (0, 800),
        (5_000, 5_000),
    ];
    for (x, y) in outside {
        let fake = FakeSink::new();
        let moving = plan(
            vec![
                Step::Key {
                    code: 38,
                    down: true,
                },
                Step::MoveTo { x, y },
            ],
            None,
        );

        let error = run_with(open(&fake), vec![moving], &mut no_gate(), deadline(5_000))
            .expect_err("outside");

        assert_eq!(error.code, ErrorCode::InvalidArgs, "({x}, {y})");
        assert!(events(&fake).is_empty(), "({x}, {y}) sent an event");
    }
}

#[test]
fn points_on_the_first_and_last_row_and_column_are_delivered() {
    for (x, y) in [(0, 0), (1279, 799), (0, 799), (1279, 0)] {
        let fake = FakeSink::new();
        let moving = plan(vec![Step::MoveTo { x, y }], None);

        run_with(open(&fake), vec![moving], &mut no_gate(), deadline(5_000)).expect("inside");

        assert_eq!(events(&fake), [Recorded::Move(x as i16, y as i16)]);
    }
}

#[test]
fn a_button_still_held_after_the_call_fails_it_loudly() {
    let fake = FakeSink::new();
    fake.world().sticky_buttons.push(1);
    let click = plan(
        vec![
            Step::Button {
                code: 1,
                down: true,
            },
            Step::Button {
                code: 1,
                down: false,
            },
        ],
        None,
    );

    let error =
        run_with(open(&fake), vec![click], &mut no_gate(), deadline(5_000)).expect_err("held");

    assert_eq!(error.details.expect("details")["leftover_buttons"][0], 1);
}

#[test]
fn a_button_held_by_someone_else_does_not_fail_the_call() {
    let fake = FakeSink::new();
    fake.world().sticky_buttons.push(3);
    let click = plan(
        vec![
            Step::Button {
                code: 1,
                down: true,
            },
            Step::Button {
                code: 1,
                down: false,
            },
        ],
        None,
    );

    run_with(open(&fake), vec![click], &mut no_gate(), deadline(5_000)).expect("delivered");
}

#[test]
fn a_connection_that_cannot_open_reports_its_own_error_and_sends_nothing() {
    let refusal = || -> Result<FakeSink, AdapterError> {
        Err(AdapterError::new(ErrorCode::ActionNotSupported, "no XTEST"))
    };

    let error =
        run_with(refusal, vec![drag_plan()], &mut no_gate(), deadline(5_000)).expect_err("refused");

    assert_eq!(error.code, ErrorCode::ActionNotSupported);
}

#[test]
fn a_remap_is_held_for_the_settle_time_before_it_is_restored() {
    use crate::input::keymap::KeymapEdit;
    use std::time::Instant;
    let fake = FakeSink::new();
    let typed = plan(
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
    );
    let started = Instant::now();

    run_with(open(&fake), vec![typed], &mut no_gate(), deadline(5_000)).expect("typed");

    assert!(
        started.elapsed() >= std::time::Duration::from_millis(100),
        "the remap was restored before the keys were consumed"
    );
}
