/// Whether the compositor keeps a toplevel's buffer current, which decides
/// which frame of it a capture may answer with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Composition {
    /// The compositor composes the toplevel for an output and asks it for
    /// each frame, so its buffer is the newest it drew and the first frame
    /// of a session is the answer.
    Current,
    /// The compositor composes the toplevel for no output, so its buffer can
    /// be older than the request and a capture waits for a redraw.
    Stale,
}
