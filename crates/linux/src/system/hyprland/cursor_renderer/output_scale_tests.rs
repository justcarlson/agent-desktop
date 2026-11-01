use super::OutputScale;
use crate::system::hyprland::cursor_renderer::output_geometry::OutputGeometry;

fn geometry() -> OutputGeometry {
    OutputGeometry {
        origin: Some((-1024, -640)),
        size: Some((1024, 640)),
        mode: Some((1280, 800)),
        turned: false,
    }
}

#[test]
fn a_fractional_preference_wins_over_every_other_hint() {
    let scale = OutputScale {
        fractional: Some(180),
        buffer: Some(2),
        integer: Some(2),
    };
    assert_eq!(scale.resolve(&geometry(), true), 1.5);
}

#[test]
fn the_buffer_scale_counts_only_without_a_fractional_manager() {
    let scale = OutputScale {
        fractional: None,
        buffer: Some(2),
        integer: Some(1),
    };
    assert_eq!(scale.resolve(&geometry(), false), 2.0);
    assert_eq!(scale.resolve(&geometry(), true), 1.25);
}

#[test]
fn the_output_mode_over_its_logical_size_comes_before_the_integer_scale() {
    let scale = OutputScale {
        integer: Some(2),
        ..OutputScale::default()
    };
    assert_eq!(scale.resolve(&geometry(), true), 1.25);
    let turned = OutputGeometry {
        mode: Some((800, 1280)),
        turned: true,
        ..geometry()
    };
    assert_eq!(scale.resolve(&turned, true), 1.25);
    assert_eq!(scale.resolve(&OutputGeometry::default(), true), 2.0);
}
