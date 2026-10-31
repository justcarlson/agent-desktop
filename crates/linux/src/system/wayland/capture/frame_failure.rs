/// Why the compositor failed one capture frame. Screencopy's `failed`
/// carries no reason, so every screencopy failure reads as `Unknown`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FrameFailure {
    Unknown,
}
