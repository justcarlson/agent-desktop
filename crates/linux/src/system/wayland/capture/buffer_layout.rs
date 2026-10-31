/// The shape of one shared-memory frame buffer: its pixel format, size and
/// row stride in bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BufferLayout {
    pub(crate) format: u32,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) stride: u32,
}
