use agent_desktop_core::{AdapterError, DeliveryDisposition, ErrorCode};

use crate::input::plan::Plan;
use crate::input::step::Step;
use crate::input::x11::fake_sink::{FakeSink, Recorded, deadline, events, run};

#[test]
fn a_gate_that_refuses_the_first_plan_is_not_delivered() {
    let fake = FakeSink::new();
    let only = Plan {
        steps: vec![
            Step::Key {
                code: 38,
                down: true,
            },
            Step::Key {
                code: 38,
                down: false,
            },
        ],
        release_origin: None,
    };
    let mut gate = || Err(AdapterError::new(ErrorCode::ActionFailed, "not active"));

    let error = run(&fake, vec![only], &mut gate, deadline(5_000), None).expect_err("refused");

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
        let moving = Plan {
            steps: vec![
                Step::Key {
                    code: 38,
                    down: true,
                },
                Step::MoveTo { x, y },
            ],
            release_origin: None,
        };

        let error =
            run(&fake, vec![moving], &mut || Ok(()), deadline(5_000), None).expect_err("outside");

        assert_eq!(error.code, ErrorCode::InvalidArgs, "({x}, {y})");
        assert!(events(&fake).is_empty(), "({x}, {y}) sent an event");
    }
    for (x, y) in [(0, 0), (1279, 799), (0, 799), (1279, 0)] {
        let fake = FakeSink::new();
        let moving = Plan {
            steps: vec![Step::MoveTo { x, y }],
            release_origin: None,
        };

        run(&fake, vec![moving], &mut || Ok(()), deadline(5_000), None).expect("inside");

        assert_eq!(events(&fake), [Recorded::Move(x as i16, y as i16)]);
    }
}

#[test]
fn owned_sticky_input_is_reported_without_claiming_bystander_input() {
    let fake = FakeSink::new();
    fake.world().sticky_buttons.push(1);
    fake.world().sticky_buttons.push(3);
    fake.world().sticky_keys.push(38);
    fake.world().sticky_keys.push(39);
    let click = Plan {
        steps: vec![
            Step::Key {
                code: 38,
                down: true,
            },
            Step::Key {
                code: 38,
                down: false,
            },
            Step::Button {
                code: 1,
                down: true,
            },
            Step::Button {
                code: 1,
                down: false,
            },
        ],
        release_origin: None,
    };

    let error = run(&fake, vec![click], &mut || Ok(()), deadline(5_000), None).expect_err("held");

    assert_eq!(
        error.disposition.delivery(),
        DeliveryDisposition::DeliveredUnverified
    );
    let details = error.details.expect("details");
    assert_eq!(details["leftover_buttons"], serde_json::json!([1]));
    assert_eq!(details["leftover_keys"], serde_json::json!([38]));
}

#[test]
fn the_worker_runs_the_planned_hold_between_the_keys_and_the_restore() {
    use crate::input::keymap_edit::KeymapEdit;
    use std::time::Instant;
    let fake = FakeSink::new();
    let typed = Plan {
        steps: vec![
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
            Step::Pause(std::time::Duration::from_millis(50)),
            Step::Keymap(KeymapEdit {
                keycode: 12,
                keysyms: vec![0, 0],
            }),
        ],
        release_origin: None,
    };
    let started = Instant::now();

    run(&fake, vec![typed], &mut || Ok(()), deadline(5_000), None).expect("typed");

    assert!(
        started.elapsed() >= std::time::Duration::from_millis(50),
        "the remap was restored before the keys were consumed"
    );
}
