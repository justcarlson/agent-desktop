/// Why the compositor failed one capture frame, as
/// `ext_image_copy_capture_frame_v1.failed` names it. Screencopy's `failed`
/// carries no reason and reads as `Unknown`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FrameFailure {
    Unknown,
    BufferConstraints,
    Stopped,
}
