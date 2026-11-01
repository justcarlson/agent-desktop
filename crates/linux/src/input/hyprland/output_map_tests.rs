use agent_desktop_core::{DisplayInfo, Rect};

use super::*;

fn display(id: &str, x: f64, y: f64, size: (f64, f64), scale: f64) -> DisplayInfo {
    DisplayInfo {
        id: id.to_owned(),
        bounds: Rect {
            x,
            y,
            width: size.0,
            height: size.1,
        },
        is_primary: false,
        scale,
    }
}

fn layout() -> Vec<DisplayInfo> {
    vec![
        display("Virtual-1", 0.0, 0.0, (1024.0, 640.0), 1.25),
        display("S7NEG", -1024.0, 0.0, (1024.0, 576.0), 1.25),
        display("S7DIAG", -1280.0, -720.0, (1280.0, 720.0), 1.0),
    ]
}

fn point(output: &str, x: u32, y: u32, extent: (u32, u32)) -> Option<OutputPoint> {
    Some(OutputPoint {
        output: output.to_owned(),
        x,
        y,
        x_extent: extent.0,
        y_extent: extent.1,
    })
}

#[test]
fn a_point_at_scale_1_25_is_output_local_with_the_logical_extent() {
    assert_eq!(
        to_output(162, 238, &layout()),
        point("Virtual-1", 162 * 120, 238 * 120, (1024 * 120, 640 * 120))
    );
}

#[test]
fn a_negative_origin_output_takes_the_offset_from_its_origin() {
    assert_eq!(
        to_output(-862, 238, &layout()),
        point("S7NEG", 162 * 120, 238 * 120, (1024 * 120, 576 * 120))
    );
    assert_eq!(
        to_output(-1118, -482, &layout()),
        point("S7DIAG", 162 * 120, 238 * 120, (1280 * 120, 720 * 120))
    );
}

#[test]
fn an_output_edge_belongs_to_the_output_that_starts_there() {
    assert_eq!(
        to_output(0, 0, &layout()).map(|point| point.output),
        Some("Virtual-1".to_owned())
    );
    assert_eq!(
        to_output(-1, 0, &layout()).map(|point| (point.output, point.x)),
        Some(("S7NEG".to_owned(), 1023 * 120))
    );
}

#[test]
fn a_point_on_no_output_maps_to_none() {
    assert_eq!(to_output(1024, 10, &layout()), None);
    assert_eq!(to_output(-500, -10, &layout()), None);
    assert_eq!(to_output(10, 10, &[]), None);
}
