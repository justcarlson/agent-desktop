use std::time::Duration;

use super::*;
use crate::system::wayland::capture::frame_failure::FrameFailure;

const REQUESTED: Duration = Duration::from_secs(10);

fn record(outcome: FrameOutcome, presented: Option<Duration>) -> FrameRecord {
    FrameRecord { presented, outcome }
}

#[test]
fn a_ready_frame_presented_after_the_request_may_be_read() {
    let frame = record(
        FrameOutcome::Ready,
        Some(REQUESTED + Duration::from_millis(5)),
    );
    assert_eq!(settle_frame(Ok(()), frame, REQUESTED), Ok(()));
}

#[test]
fn a_ready_frame_presented_before_the_request_or_untimed_is_stale() {
    let early = record(
        FrameOutcome::Ready,
        Some(REQUESTED - Duration::from_millis(1)),
    );
    assert_eq!(
        settle_frame(Ok(()), early, REQUESTED),
        Err(CaptureFault::Stale)
    );
    let untimed = record(FrameOutcome::Ready, None);
    assert_eq!(
        settle_frame(Ok(()), untimed, REQUESTED),
        Err(CaptureFault::Stale)
    );
}

#[test]
fn a_failed_frame_keeps_its_reason() {
    for failure in [
        FrameFailure::Unknown,
        FrameFailure::BufferConstraints,
        FrameFailure::Stopped,
    ] {
        let frame = record(FrameOutcome::Failed(failure), None);
        assert_eq!(
            settle_frame(Ok(()), frame, REQUESTED),
            Err(CaptureFault::Failed(failure))
        );
    }
}

#[test]
fn a_silent_or_lost_compositor_is_never_read_even_with_an_earlier_ready_record() {
    let ready = record(
        FrameOutcome::Ready,
        Some(REQUESTED + Duration::from_millis(5)),
    );
    assert_eq!(
        settle_frame(Err(WaylandFault::Silent), ready, REQUESTED),
        Err(CaptureFault::FrameSilent)
    );
    assert_eq!(
        settle_frame(Err(WaylandFault::Gone), ready, REQUESTED),
        Err(CaptureFault::FrameGone)
    );
    let pending = record(FrameOutcome::Pending, None);
    assert_eq!(
        settle_frame(Ok(()), pending, REQUESTED),
        Err(CaptureFault::FrameSilent)
    );
}
