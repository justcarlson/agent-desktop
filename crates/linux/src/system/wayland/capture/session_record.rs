/// What an `ext_image_copy_capture_session_v1` announced: the buffer size,
/// the shared-memory formats it accepts, and whether its constraints are
/// complete or the session stopped.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SessionRecord {
    pub(crate) size: Option<(u32, u32)>,
    pub(crate) formats: Vec<u32>,
    pub(crate) done: bool,
    pub(crate) stopped: bool,
}
