use super::*;

fn output(
    mode: Option<(i32, i32)>,
    logical: (i32, i32),
    turned: bool,
    integer: i32,
) -> WaylandOutput {
    WaylandOutput {
        name: Some("DP-1".to_owned()),
        position: Some((0, 0)),
        logical: Some(logical),
        mode,
        turned,
        integer_scale: integer,
    }
}

#[test]
fn a_fractional_scale_is_the_mode_width_over_the_logical_width() {
    assert_eq!(
        output(Some((2880, 1800)), (1920, 1200), false, 2).scale(),
        1.5
    );
    assert_eq!(
        output(Some((1920, 1080)), (1920, 1080), false, 1).scale(),
        1.0
    );
}

#[test]
fn a_turned_output_measures_its_mode_height_against_the_logical_width() {
    assert_eq!(
        output(Some((3840, 2160)), (1440, 2560), true, 2).scale(),
        1.5
    );
}

#[test]
fn a_scale_from_rounded_sizes_lands_on_the_nearest_hundred_twentieth() {
    let scale = output(Some((2560, 1440)), (1921, 1081), false, 2).scale();
    assert_eq!(scale, 160.0 / 120.0);
}

#[test]
fn an_output_without_a_mode_reports_its_integer_scale_and_never_below_one() {
    assert_eq!(output(None, (1920, 1080), false, 2).scale(), 2.0);
    assert_eq!(output(None, (1920, 1080), false, 0).scale(), 1.0);
}

#[test]
fn the_display_carries_the_connector_name_and_logical_bounds() {
    let mut source = output(Some((2880, 1800)), (1920, 1200), false, 2);
    source.position = Some((-1920, 40));
    let display = source.display().expect("complete");
    assert_eq!(display.id, "DP-1");
    assert_eq!(
        (
            display.bounds.x,
            display.bounds.y,
            display.bounds.width,
            display.bounds.height
        ),
        (-1920.0, 40.0, 1920.0, 1200.0)
    );
    assert_eq!(display.scale, 1.5);
    assert!(!display.is_primary);
}

#[test]
fn an_output_without_a_name_position_or_size_is_no_display() {
    let complete = output(None, (1920, 1080), false, 1);
    let unnamed = WaylandOutput {
        name: None,
        ..complete.clone()
    };
    let blank = WaylandOutput {
        name: Some(String::new()),
        ..complete.clone()
    };
    let unplaced = WaylandOutput {
        position: None,
        ..complete.clone()
    };
    let empty = WaylandOutput {
        logical: Some((0, 1080)),
        ..complete.clone()
    };
    assert!(complete.display().is_some());
    for broken in [unnamed, blank, unplaced, empty] {
        assert!(broken.display().is_none(), "{broken:?}");
    }
}
