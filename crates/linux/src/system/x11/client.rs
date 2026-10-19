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
}
