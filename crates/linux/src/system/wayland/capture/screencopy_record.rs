use super::buffer_layout::BufferLayout;

/// What a `zwlr_screencopy_frame_v1` announced before the copy: the buffer
/// layouts it accepts, whether the list is complete, and whether the copy
/// will arrive upside down.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ScreencopyRecord {
    pub(crate) offers: Vec<BufferLayout>,
    pub(crate) offers_done: bool,
    pub(crate) inverted: bool,
}
