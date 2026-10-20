use super::FrameExtents;
use crate::tree::Extents;

/// One top-level window the window manager manages. `bounds` is the client
/// rectangle in root coordinates, inside the frame `extents` describes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct X11Client {
    pub(crate) id: u32,
    pub(crate) pid: Option<u32>,
    pub(crate) title: String,
    pub(crate) extents: Option<FrameExtents>,
    pub(crate) bounds: Option<Extents>,
    pub(crate) hidden: bool,
    /// The left and top of `_GTK_FRAME_EXTENTS`: the client-side shadow a
    /// GTK window draws inside its client, which the toolkit's window
    /// coordinates start after. Zero for a window with no shadow.
    pub(crate) shadow: (i32, i32),
}
