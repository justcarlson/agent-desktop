use super::frame_failure::FrameFailure;
use crate::system::wayland::wayland_fault::WaylandFault;

/// Why a capture on the compositor's own protocols returned no pixels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CaptureFault {
    /// The connection broke or went quiet before the frame was requested.
    Setup(WaylandFault),
    /// The compositor accepted the request and sent no frame in time.
    FrameSilent,
    /// The compositor closed the connection while the frame was pending.
    FrameGone,
    Unsupported(&'static str),
    NoFormat,
    Failed(FrameFailure),
    /// The frame carries no presentation time after the request.
    Stale,
    OutputMissing(String),
}
