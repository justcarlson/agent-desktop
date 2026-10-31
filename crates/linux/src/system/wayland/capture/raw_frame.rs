/// The pixels of one captured frame as the compositor wrote them into
/// shared memory, with the layout needed to read them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RawFrame {
    pub(crate) data: Vec<u8>,
    pub(crate) format: u32,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) stride: u32,
    pub(crate) inverted: bool,
}
