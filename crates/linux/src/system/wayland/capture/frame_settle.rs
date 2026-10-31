use std::time::Duration;

use super::capture_fault::CaptureFault;
use super::frame_freshness::is_fresh;
use super::frame_outcome::FrameOutcome;
use super::frame_record::FrameRecord;
use crate::system::wayland::wayland_fault::WaylandFault;

/// Whether a waited-for frame may be read: it must have arrived, been
/// filled, and been presented no earlier than `requested`. A frame that
/// never answered is silent, never a cached image.
pub(crate) fn settle_frame(
    waited: Result<(), WaylandFault>,
    frame: FrameRecord,
    requested: Duration,
) -> Result<(), CaptureFault> {
    match waited {
        Err(WaylandFault::Silent) => return Err(CaptureFault::FrameSilent),
        Err(WaylandFault::Gone) => return Err(CaptureFault::FrameGone),
        Err(fault @ WaylandFault::Protocol(_)) => return Err(CaptureFault::Setup(fault)),
        Ok(()) => {}
    }
    match frame.outcome {
        FrameOutcome::Ready if is_fresh(requested, frame.presented) => Ok(()),
        FrameOutcome::Ready => Err(CaptureFault::Stale),
        FrameOutcome::Failed(failure) => Err(CaptureFault::Failed(failure)),
        FrameOutcome::Pending => Err(CaptureFault::FrameSilent),
    }
}

#[cfg(test)]
#[path = "frame_settle_tests.rs"]
mod tests;
