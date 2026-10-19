use super::*;

fn rect(x: i32, y: i32, width: i32, height: i32) -> Extents {
    Extents {
        x,
        y,
        width,
        height,
    }
}

const VIEWPORT: Extents = Extents {
    x: 0,
    y: 0,
    width: 420,
    height: 420,
};

fn bar(current: f64) -> BarRange {
    BarRange {
        min: 0.0,
        max: 10_482.0,
        current,
    }
}

fn gtk() -> ToolkitId {
    ToolkitId {
        name: Some("GTK".to_owned()),
        version: Some("3.24.41".to_owned()),
    }
}

#[test]
fn a_target_far_below_gets_a_write_that_brings_its_top_to_the_viewport_top() {
    let step = next_value(
        bar(0.0),
        VIEWPORT,
        Axis::Vertical,
        Aim::Rect(rect(2, 5526, 420, 22)),
    );
    assert_eq!(step, Step::Write(5526.0));
}

#[test]
fn a_write_is_clamped_to_the_maximum() {
    let step = next_value(
        bar(10_000.0),
        VIEWPORT,
        Axis::Vertical,
        Aim::Rect(rect(2, 5526, 420, 22)),
    );
    assert_eq!(step, Step::Write(10_482.0));
}

#[test]
fn a_target_above_gets_a_smaller_value() {
    let step = next_value(
        bar(8_000.0),
        VIEWPORT,
        Axis::Vertical,
        Aim::Rect(rect(2, -3000, 420, 22)),
    );
    assert_eq!(step, Step::Write(5_000.0));
}

#[test]
fn a_bar_already_at_the_limit_in_the_needed_direction_writes_nothing() {
    let forward = next_value(
        bar(10_482.0),
        VIEWPORT,
        Axis::Vertical,
        Aim::Rect(rect(2, 5526, 420, 22)),
    );
    assert_eq!(forward, Step::AtLimit);
    let backward = next_value(
        bar(0.0),
        VIEWPORT,
        Axis::Vertical,
        Aim::Rect(rect(2, -500, 420, 22)),
    );
    assert_eq!(backward, Step::AtLimit);
}

#[test]
fn a_target_without_a_position_moves_by_one_page_in_its_sibling_direction() {
    let ahead = Aim::Index {
        delta: 250,
        unit: None,
        anchor: 0,
    };
    assert_eq!(
        next_value(bar(1_000.0), VIEWPORT, Axis::Vertical, ahead),
        Step::Write(1_420.0)
    );
    let behind = Aim::Index {
        delta: -3,
        unit: None,
        anchor: 0,
    };
    assert_eq!(
        next_value(bar(1_000.0), VIEWPORT, Axis::Vertical, behind),
        Step::Write(580.0)
    );
}

#[test]
fn a_sibling_distance_with_a_known_unit_moves_by_that_many_units_from_the_anchor() {
    let aim = Aim::Index {
        delta: 100,
        unit: Some(40),
        anchor: 30,
    };
    assert_eq!(
        next_value(bar(0.0), VIEWPORT, Axis::Vertical, aim),
        Step::Write(4_030.0)
    );
}

#[test]
fn the_axis_is_horizontal_only_when_the_target_is_off_sideways_alone() {
    assert_eq!(
        axis_of(Some(rect(900, 10, 50, 20)), Some(VIEWPORT)),
        Axis::Horizontal
    );
    assert_eq!(
        axis_of(Some(rect(10, 900, 50, 20)), Some(VIEWPORT)),
        Axis::Vertical
    );
    assert_eq!(
        axis_of(Some(rect(900, 900, 50, 20)), Some(VIEWPORT)),
        Axis::Vertical
    );
}

#[test]
fn gtk_goes_straight_to_the_bar_and_every_other_toolkit_tries_scroll_to_first() {
    assert_eq!(rungs_for(&gtk()), &[Rung::BarValue]);
    let chromium = ToolkitId {
        name: Some("Chromium".to_owned()),
        version: None,
    };
    assert_eq!(rungs_for(&chromium), &[Rung::ScrollTo, Rung::BarValue]);
    assert_eq!(
        rungs_for(&ToolkitId::default()),
        &[Rung::ScrollTo, Rung::BarValue]
    );
}

#[test]
fn a_target_with_a_position_aims_by_its_rectangle_and_one_without_by_its_sibling() {
    let target = rect(2, 5526, 420, 22);
    assert_eq!(
        aim_for(Some(target), None, Axis::Vertical, VIEWPORT),
        Some(Aim::Rect(target))
    );
    let sentinel = rect(i32::MIN, i32::MIN, 420, 22);
    let sibling = rect(0, 60, 420, 40);
    assert_eq!(
        aim_for(
            Some(sentinel),
            Some((250, sibling)),
            Axis::Vertical,
            VIEWPORT
        ),
        Some(Aim::Index {
            delta: 250,
            unit: Some(40),
            anchor: 60
        })
    );
    assert_eq!(
        aim_for(Some(sentinel), None, Axis::Vertical, VIEWPORT),
        None
    );
}

#[test]
fn a_target_or_viewport_that_cannot_be_measured_runs_the_vertical_axis() {
    assert_eq!(axis_of(None, Some(VIEWPORT)), Axis::Vertical);
    assert_eq!(axis_of(Some(rect(900, 10, 5, 5)), None), Axis::Vertical);
}
