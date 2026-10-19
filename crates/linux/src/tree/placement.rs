use super::Extents;
use crate::system::toolkit_id::ToolkitId;
use crate::system::x11::window_x11_state::WindowX11State;

/// Where a node sits relative to its window, as far as it could be read.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct Placement {
    pub(crate) node: Option<Extents>,
    pub(crate) frame: Option<Extents>,
    pub(crate) container: Option<Extents>,
    pub(crate) frame_iconified: bool,
    pub(crate) toolkit: ToolkitId,
    pub(crate) x11: Option<WindowX11State>,
    pub(crate) displays: Option<Vec<Extents>>,
}
