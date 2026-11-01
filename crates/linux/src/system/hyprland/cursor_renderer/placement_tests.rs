use agent_desktop_core::Point;

use super::{Placement, place};
use crate::system::hyprland::cursor_renderer::arrow_frame::ArrowFrame;
use crate::system::hyprland::cursor_renderer::output_layout::OutputLayout;

fn frame(scale: f64) -> ArrowFrame {
    ArrowFrame {
        size: (
            (22.0 * scale).ceil() as u32 + 2,
            (30.0 * scale).ceil() as u32 + 2,
        ),
        hotspot: (1.0, 1.0),
    }
}

fn output(name: u32, origin: (f64, f64), size: (f64, f64), scale: f64) -> OutputLayout {
    OutputLayout {
        output: name,
        origin,
        size,
        scale,
    }
}

fn tip_error(point: &Point, on: &OutputLayout, placed: &Placement) -> (f64, f64) {
    let hotspot = frame(on.scale).hotspot;
    let axis = |target: f64, origin: f64, margin: i32, offset: i32, hot: f64| {
        let drawn = (f64::from(margin) * on.scale).round() + f64::from(offset) + hot;
        (drawn - (target - origin) * on.scale).abs()
    };
    (
        axis(
            point.x,
            on.origin.0,
            placed.margin.0,
            placed.offset.0,
            hotspot.0,
        ),
        axis(
            point.y,
            on.origin.1,
            placed.margin.1,
            placed.offset.1,
            hotspot.1,
        ),
    )
}

struct Row {
    on: OutputLayout,
    point: Point,
    margin: (i32, i32),
    logical: (i32, i32),
    buffer: (i32, i32),
    offset: (i32, i32),
}

fn table() -> Vec<Row> {
    vec![
        Row {
            on: output(1, (0.0, 0.0), (1280.0, 800.0), 1.0),
            point: Point { x: 100.4, y: 200.6 },
            margin: (99, 199),
            logical: (25, 33),
            buffer: (25, 33),
            offset: (0, 1),
        },
        Row {
            on: output(2, (-1024.0, -640.0), (1024.0, 640.0), 1.25),
            point: Point {
                x: -862.0,
                y: -402.0,
            },
            margin: (161, 237),
            logical: (28, 36),
            buffer: (35, 45),
            offset: (1, 1),
        },
        Row {
            on: output(3, (-1920.0, 0.0), (1280.0, 720.0), 1.5),
            point: Point {
                x: -1000.3,
                y: 10.7,
            },
            margin: (919, 10),
            logical: (26, 34),
            buffer: (39, 51),
            offset: (0, 0),
        },
        Row {
            on: output(4, (0.0, -1080.0), (960.0, 1080.0), 2.0),
            point: Point { x: 10.25, y: -5.75 },
            margin: (9, 1073),
            logical: (24, 32),
            buffer: (48, 64),
            offset: (2, 2),
        },
    ]
}

#[test]
fn margin_and_buffer_follow_each_scale_and_negative_origin() {
    for row in table() {
        let placed = place(&row.point, std::slice::from_ref(&row.on), frame);
        assert_eq!(placed.len(), 1, "scale {}", row.on.scale);
        let placed = placed[0];
        assert_eq!(placed.output, Some(row.on.output));
        assert_eq!(placed.margin, row.margin, "scale {}", row.on.scale);
        assert_eq!(placed.logical, row.logical, "scale {}", row.on.scale);
        assert_eq!(placed.buffer, row.buffer, "scale {}", row.on.scale);
        assert_eq!(placed.offset, row.offset, "scale {}", row.on.scale);
        assert_eq!(placed.scale, row.on.scale);
    }
}

#[test]
fn the_buffer_is_the_logical_size_times_the_scale_rounded_up_and_holds_the_arrow() {
    for row in table() {
        let placed = place(&row.point, std::slice::from_ref(&row.on), frame)[0];
        let scaled = |logical: i32| (f64::from(logical) * row.on.scale).ceil() as i32;
        assert_eq!(placed.buffer.0, scaled(placed.logical.0));
        assert_eq!(placed.buffer.1, scaled(placed.logical.1));
        let arrow = frame(row.on.scale).size;
        assert!(placed.buffer.0 >= arrow.0 as i32 + placed.offset.0);
        assert!(placed.buffer.1 >= arrow.1 as i32 + placed.offset.1);
    }
}

#[test]
fn the_hotspot_lands_within_one_physical_pixel_of_the_point() {
    for row in table() {
        for step in 0..8 {
            let nudge = f64::from(step) / 8.0;
            let point = Point {
                x: row.point.x + nudge,
                y: row.point.y - nudge,
            };
            let placed = place(&point, std::slice::from_ref(&row.on), frame)[0];
            let (x, y) = tip_error(&point, &row.on, &placed);
            assert!(
                x <= 1.0 && y <= 1.0,
                "scale {} at {point:?}: off by {x} {y}",
                row.on.scale
            );
        }
    }
}

#[test]
fn a_box_crossing_an_output_edge_gets_one_surface_per_output_at_its_scale() {
    let left = output(5, (-1024.0, 0.0), (1024.0, 768.0), 1.25);
    let right = output(6, (0.0, 0.0), (1280.0, 800.0), 1.0);
    let point = Point { x: -5.0, y: 100.0 };
    let placed = place(&point, &[left.clone(), right.clone()], frame);
    assert_eq!(placed.len(), 2);
    let on_left = placed[0];
    let on_right = placed[1];
    assert_eq!(on_left.output, Some(5));
    assert_eq!(on_left.margin, (1018, 99));
    assert_eq!(on_left.scale, 1.25);
    assert_eq!(on_right.output, Some(6));
    assert_eq!(on_right.margin, (-7, 99));
    assert_eq!(on_right.buffer, (25, 33));
    assert_eq!(on_right.scale, 1.0);
    for (on, placed) in [(&left, &on_left), (&right, &on_right)] {
        let (x, y) = tip_error(&point, on, placed);
        assert!(x <= 1.0 && y <= 1.0, "output {}: off by {x} {y}", on.output);
    }
}

#[test]
fn a_box_inside_one_output_touches_only_that_output() {
    let left = output(5, (-1024.0, 0.0), (1024.0, 768.0), 1.25);
    let right = output(6, (0.0, 0.0), (1280.0, 800.0), 1.0);
    let placed = place(&Point { x: 500.0, y: 500.0 }, &[left, right], frame);
    assert_eq!(placed.len(), 1);
    assert_eq!(placed[0].output, Some(6));
}

#[test]
fn a_point_outside_every_output_places_nothing() {
    let right = output(6, (0.0, 0.0), (1280.0, 800.0), 1.0);
    assert!(
        place(
            &Point {
                x: 5000.0,
                y: 5000.0
            },
            &[right],
            frame
        )
        .is_empty()
    );
}

#[test]
fn with_no_output_layout_one_unpinned_surface_draws_at_scale_one() {
    let placed = place(&Point { x: 100.4, y: 200.6 }, &[], frame);
    assert_eq!(placed.len(), 1);
    assert_eq!(placed[0].output, None);
    assert_eq!(placed[0].margin, (99, 199));
    assert_eq!(placed[0].scale, 1.0);
}
