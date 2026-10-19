use agent_desktop_core::Deadline;

use super::FrameFacts;
use super::stacking_source::StackingSource;
use super::window_x11_state::WindowX11State;

/// The X11 client of a listed window and its state, or `None` when the
/// session is not X11, no manager answers, or the window maps to no single
/// client.
#[expect(dead_code, reason = "stub the owning lane replaces")]
pub(crate) fn frame_window_state(
    _source: &dyn StackingSource,
    _deadline: Deadline,
    _frame: &FrameFacts,
) -> Option<WindowX11State> {
    None
}
