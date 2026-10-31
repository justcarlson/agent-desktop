use super::frame_failure::FrameFailure;

/// Where one capture frame stands: no answer yet, filled, or failed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum FrameOutcome {
    #[default]
    Pending,
    Ready,
    Failed(FrameFailure),
}
