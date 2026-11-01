use agent_desktop_core::{CursorOverlayControl, CursorOverlayStyle};

use super::ScaledArrows;
use crate::system::hyprland::cursor_renderer::agent_cursor::AgentCursor;

#[test]
fn the_largest_style_on_a_scale_3_output_draws_three_times_its_scale_1_arrow() {
    let mut style = CursorOverlayStyle::default();
    style.set_size(4.0);
    let cursor = AgentCursor::new(&CursorOverlayControl::enable("s".into(), style));
    let arrow = ScaledArrows::default().at(&cursor, 3.0);
    assert_eq!(cursor.bitmap.width, (22.0_f64 * 4.0).ceil() as u32 + 2);
    assert_eq!(arrow.width, (22.0_f64 * 12.0).ceil() as u32 + 2);
    assert_eq!(arrow.height, (30.0_f64 * 12.0).ceil() as u32 + 2);
}
