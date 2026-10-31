use super::*;
use crate::system::wayland::capture::frame_failure::FrameFailure;

fn frame(fill: u8) -> RawFrame {
    RawFrame {
        data: vec![fill; 16],
        format: 1,
        width: 2,
        height: 2,
        stride: 8,
        inverted: false,
    }
}

#[test]
fn the_redrawn_frame_replaces_the_first() {
    assert_eq!(redrawn(frame(1), Ok(frame(2))), Ok(frame(2)));
}

#[test]
fn a_withheld_redraw_leaves_the_first_frame() {
    assert_eq!(
        redrawn(frame(1), Err(CaptureFault::FrameSilent)),
        Ok(frame(1))
    );
}

#[test]
fn any_other_failure_of_the_redraw_fails_the_capture() {
    for fault in [
        CaptureFault::FrameGone,
        CaptureFault::Stale,
        CaptureFault::Failed(FrameFailure::Stopped),
        CaptureFault::Failed(FrameFailure::BufferConstraints),
        CaptureFault::Failed(FrameFailure::Unknown),
    ] {
        assert_eq!(redrawn(frame(1), Err(fault.clone())), Err(fault));
    }
}
