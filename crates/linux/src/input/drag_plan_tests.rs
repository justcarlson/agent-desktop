use super::*;

fn params(duration_ms: Option<u64>, drop_delay_ms: Option<u64>) -> DragParams {
    DragParams {
        from: Point { x: 10.0, y: 10.0 },
        to: Point { x: 110.0, y: 50.0 },
        duration_ms,
        drop_delay_ms,
    }
}

fn deadline(ms: u64) -> Deadline {
    Deadline::after(ms).unwrap()
}

fn pauses(plan: &Plan) -> Duration {
    plan.steps
        .iter()
        .filter_map(|step| match step {
            Step::Pause(duration) => Some(*duration),
            _ => None,
        })
        .sum()
}

#[test]
fn the_plan_starts_at_the_origin_and_ends_with_a_release_at_the_destination() {
    let plan = plan_drag(&params(Some(300), Some(100)), deadline(5_000)).unwrap();
    assert_eq!(plan.steps.first(), Some(&Step::MoveTo { x: 10, y: 10 }));
    let release = plan.steps.len() - 1;
    assert_eq!(
        plan.steps[release],
        Step::Button {
            code: BUTTON_LEFT,
            down: false
        }
    );
    let last_move = plan
        .steps
        .iter()
        .rev()
        .find(|step| matches!(step, Step::MoveTo { .. }));
    assert_eq!(last_move, Some(&Step::MoveTo { x: 110, y: 50 }));
    assert_eq!(plan.release_origin, Some((10, 10)));
}

#[test]
fn the_button_goes_down_once_after_the_pickup_pause_and_before_the_first_travel_move() {
    let plan = plan_drag(&params(Some(300), Some(0)), deadline(5_000)).unwrap();
    let down = plan
        .steps
        .iter()
        .position(|step| {
            *step
                == Step::Button {
                    code: BUTTON_LEFT,
                    down: true,
                }
        })
        .unwrap();
    assert_eq!(
        plan.steps[down - 1],
        Step::Pause(Duration::from_millis(PICKUP_DELAY_MS))
    );
    assert!(matches!(plan.steps[down + 2], Step::MoveTo { .. }));
    let downs = plan
        .steps
        .iter()
        .filter(|step| matches!(step, Step::Button { down: true, .. }))
        .count();
    assert_eq!(downs, 1);
}

#[test]
fn travel_is_interpolated_in_sixteen_millisecond_ticks_and_reaches_the_destination() {
    let plan = plan_drag(&params(Some(320), Some(0)), deadline(5_000)).unwrap();
    let moves: Vec<_> = plan
        .steps
        .iter()
        .filter(|step| matches!(step, Step::MoveTo { .. }))
        .collect();
    assert_eq!(moves.len(), 1 + 20);
    assert_eq!(moves[1], &Step::MoveTo { x: 15, y: 12 });
}

#[test]
fn the_dwell_holds_over_the_destination_for_the_drop_delay_before_release() {
    let plan = plan_drag(&params(Some(16), Some(500)), deadline(5_000)).unwrap();
    let total = pauses(&plan);
    assert_eq!(total, Duration::from_millis(PICKUP_DELAY_MS * 2 + 16 + 500));
}

#[test]
fn a_zero_duration_still_travels_in_one_step() {
    let plan = plan_drag(&params(Some(0), Some(0)), deadline(5_000)).unwrap();
    assert!(plan.steps.contains(&Step::MoveTo { x: 110, y: 50 }));
}

#[test]
fn defaults_are_300_ms_of_travel_and_a_500_ms_dwell() {
    let plan = plan_drag(&params(None, None), deadline(5_000)).unwrap();
    let total = pauses(&plan).as_millis() as i64;
    let expected = (PICKUP_DELAY_MS * 2 + DEFAULT_DURATION_MS + DEFAULT_DROP_DELAY_MS) as i64;
    assert!((total - expected).abs() <= 1, "{total} vs {expected}");
}

#[test]
fn a_budget_shorter_than_pickup_travel_and_dwell_is_refused_before_any_event() {
    let error = plan_drag(&params(Some(300), Some(500)), deadline(1_000)).unwrap_err();
    assert_eq!(error.code, ErrorCode::Timeout);
    let details = error.details.unwrap();
    assert_eq!(details["physical_delivery_started"], false);
    assert_eq!(details["required_ms"], 1_200);
}

#[test]
fn a_budget_that_exactly_covers_the_plan_is_accepted() {
    assert!(plan_drag(&params(Some(300), Some(500)), deadline(2_000)).is_ok());
}

#[test]
fn timing_above_the_core_caps_is_invalid_args() {
    for bad in [
        params(Some(MAX_DRAG_DURATION_MS + 1), None),
        params(None, Some(MAX_DRAG_DROP_DELAY_MS + 1)),
    ] {
        let error = plan_drag(&bad, deadline(5_000)).unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidArgs);
    }
}

#[test]
fn a_non_finite_endpoint_is_invalid_args() {
    let mut bad = params(None, None);
    bad.to = Point {
        x: f64::INFINITY,
        y: 0.0,
    };
    assert_eq!(
        plan_drag(&bad, deadline(5_000)).unwrap_err().code,
        ErrorCode::InvalidArgs
    );
}

#[test]
fn the_dwell_sends_no_pointer_motion_after_the_last_travel_step() {
    let plan = plan_drag(&params(Some(16), Some(500)), deadline(5_000)).unwrap();
    let last_move = plan
        .steps
        .iter()
        .rposition(|step| matches!(step, Step::MoveTo { .. }))
        .unwrap();
    assert!(
        plan.steps[last_move + 1..]
            .iter()
            .all(|step| !matches!(step, Step::MoveTo { .. }))
    );
    let held: Duration = plan.steps[last_move + 1..]
        .iter()
        .filter_map(|step| match step {
            Step::Pause(duration) => Some(*duration),
            _ => None,
        })
        .sum();
    assert!(held >= Duration::from_millis(500));
}
