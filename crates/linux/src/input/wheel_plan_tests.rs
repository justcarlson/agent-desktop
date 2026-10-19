use super::*;
use crate::input::step::Step;

fn at() -> Point {
    Point { x: 10.0, y: 20.0 }
}

fn wheel(dx: f64, dy: f64) -> Result<Vec<Plan>, AdapterError> {
    plan_wheel(&Keymap::sample(), at(), dx, dy, &[])
}

fn clicks(plans: &[Plan], code: u8) -> usize {
    plans
        .iter()
        .flat_map(|plan| &plan.steps)
        .filter(|step| **step == Step::Button { code, down: true })
        .count()
}

fn pause_total(plans: &[Plan]) -> Duration {
    plans
        .iter()
        .flat_map(|plan| &plan.steps)
        .filter_map(|step| match step {
            Step::Pause(duration) => Some(*duration),
            _ => None,
        })
        .sum()
}

#[test]
fn a_positive_vertical_delta_rounds_to_steps_on_the_up_button() {
    let plans = wheel(0.0, 2.4).unwrap();
    assert_eq!(clicks(&plans, BUTTON_WHEEL_UP), 2);
    assert_eq!(clicks(&plans, BUTTON_WHEEL_DOWN), 0);
}

#[test]
fn a_small_negative_delta_never_rounds_to_zero_and_uses_the_opposite_button() {
    let plans = wheel(0.0, -0.2).unwrap();
    assert_eq!(clicks(&plans, BUTTON_WHEEL_DOWN), 1);
    assert_eq!(clicks(&plans, BUTTON_WHEEL_UP), 0);
}

#[test]
fn horizontal_deltas_use_buttons_seven_for_positive_and_six_for_negative() {
    assert_eq!(clicks(&wheel(3.0, 0.0).unwrap(), BUTTON_WHEEL_RIGHT), 3);
    assert_eq!(clicks(&wheel(-3.0, 0.0).unwrap(), BUTTON_WHEEL_LEFT), 3);
}

#[test]
fn the_sign_convention_matches_the_windows_adapter_for_the_same_delta() {
    assert_eq!(clicks(&wheel(0.0, 1.0).unwrap(), BUTTON_WHEEL_UP), 1);
    assert_eq!(clicks(&wheel(0.0, -1.0).unwrap(), BUTTON_WHEEL_DOWN), 1);
    assert_eq!(clicks(&wheel(1.0, 0.0).unwrap(), BUTTON_WHEEL_RIGHT), 1);
}

#[test]
fn a_delta_of_260_splits_into_26_events_of_ten_clicks() {
    let plans = wheel(0.0, 260.0).unwrap();
    assert_eq!(plans.len(), 26);
    for plan in &plans {
        let presses = plan
            .steps
            .iter()
            .filter(|step| matches!(step, Step::Button { down: true, .. }))
            .count();
        assert_eq!(presses, MAX_CLICKS_PER_EVENT as usize);
    }
    assert_eq!(clicks(&plans, BUTTON_WHEEL_UP), 260);
}

#[test]
fn the_pacing_of_260_clicks_fits_the_default_budget() {
    let total = pause_total(&wheel(0.0, 260.0).unwrap());
    assert!(
        total < Duration::from_millis(agent_desktop_core::DEFAULT_OPERATION_TIMEOUT_MS),
        "{total:?}"
    );
}

#[test]
fn a_delta_above_the_axis_cap_nan_or_infinity_is_invalid_args() {
    for delta in [1001.0, -1001.0, f64::NAN, f64::INFINITY] {
        assert_eq!(wheel(0.0, delta).unwrap_err().code, ErrorCode::InvalidArgs);
        assert_eq!(wheel(delta, 0.0).unwrap_err().code, ErrorCode::InvalidArgs);
    }
    assert!(wheel(0.0, 1000.0).is_ok());
}

#[test]
fn zero_on_both_axes_is_invalid_args() {
    assert_eq!(wheel(0.0, 0.0).unwrap_err().code, ErrorCode::InvalidArgs);
}

#[test]
fn every_event_moves_to_the_point_and_records_it_as_the_release_origin() {
    for plan in wheel(0.0, 15.0).unwrap() {
        assert!(plan.steps.contains(&Step::MoveTo { x: 10, y: 20 }));
        assert_eq!(plan.release_origin, Some((10, 20)));
    }
}

#[test]
fn modifiers_wrap_each_event_so_a_cut_between_events_holds_nothing() {
    let plans = plan_wheel(&Keymap::sample(), at(), 0.0, 15.0, &[Modifier::Ctrl]).unwrap();
    assert_eq!(plans.len(), 2);
    for plan in plans {
        assert_eq!(
            plan.steps.first(),
            Some(&Step::Key {
                code: 9,
                down: true
            })
        );
        assert_eq!(
            plan.steps.last(),
            Some(&Step::Key {
                code: 9,
                down: false
            })
        );
    }
}

#[test]
fn both_axes_plan_vertical_first_then_horizontal() {
    let plans = wheel(2.0, 3.0).unwrap();
    assert_eq!(plans.len(), 2);
    assert!(plans[0].steps.contains(&Step::Button {
        code: BUTTON_WHEEL_UP,
        down: true
    }));
    assert!(plans[1].steps.contains(&Step::Button {
        code: BUTTON_WHEEL_RIGHT,
        down: true
    }));
}
