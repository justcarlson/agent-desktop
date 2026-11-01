/// The user data of a `wl_surface.frame` callback: the number of the frame
/// it marks, so a `done` event names which commit the compositor showed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FrameMark(pub(crate) u32);
