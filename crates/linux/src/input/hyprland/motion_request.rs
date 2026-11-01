//! One virtual pointer request in a motion to an output point.

/// What the sink sends, in order, for one motion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MotionRequest {
    /// A relative motion of zero pixels.
    StillRelative,
    /// An absolute motion to `x` at the point's row.
    Absolute {
        x: u32,
    },
    Frame,
}
