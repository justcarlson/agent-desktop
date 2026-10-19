use super::*;

fn rect(x: i32, y: i32, width: i32, height: i32) -> Extents {
    Extents {
        x,
        y,
        width,
        height,
    }
}

#[test]
fn a_zero_axis_rectangle_admits_nothing() {
    assert!(!admits(rect(0, 0, 710, 0), (5, 0)));
    assert!(!admits(rect(0, 0, 0, 0), (0, 0)));
    assert!(!admits(rect(0, 0, 0, 20), (0, 5)));
}

#[test]
fn a_negative_size_admits_nothing() {
    assert!(!admits(rect(-1, -1, -1, -1), (-1, -1)));
    assert!(!admits(rect(10, 10, -5, 20), (8, 12)));
}

#[test]
fn the_sentinel_position_admits_nothing() {
    assert!(!admits(
        rect(i32::MIN, i32::MIN, 1, 1),
        (i32::MIN, i32::MIN)
    ));
    assert!(!admits(rect(i32::MIN, 5, 100, 100), (0, 6)));
}

#[test]
fn a_point_outside_the_rectangle_is_refused() {
    let button = rect(100, 100, 80, 24);
    assert!(!admits(button, (220, 110)));
    assert!(!admits(button, (180, 110)));
    assert!(!admits(button, (99, 110)));
    assert!(!admits(button, (110, 124)));
}

#[test]
fn a_point_inside_a_positive_area_rectangle_is_admitted() {
    let button = rect(100, 100, 80, 24);
    assert!(admits(button, (100, 100)));
    assert!(admits(button, (140, 112)));
    assert!(admits(button, (179, 123)));
}

#[test]
fn pixels_floor_and_reject_non_finite_values() {
    assert_eq!(pixel(310.9, 154.0), Some((310, 154)));
    assert_eq!(pixel(-0.5, 2.0), Some((-1, 2)));
    assert_eq!(pixel(f64::NAN, 1.0), None);
    assert_eq!(pixel(1.0, f64::INFINITY), None);
    assert_eq!(pixel(1e12, 1.0), None);
}
