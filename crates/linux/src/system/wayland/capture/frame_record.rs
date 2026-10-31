use std::time::Duration;

use super::frame_outcome::FrameOutcome;

/// What the compositor said about one frame: when it presented the pixels,
/// on the monotonic clock, and how the frame ended.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct FrameRecord {
    pub(crate) presented: Option<Duration>,
    pub(crate) outcome: FrameOutcome,
}
