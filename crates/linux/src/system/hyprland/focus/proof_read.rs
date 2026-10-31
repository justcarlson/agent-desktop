/// One read of every focus witness for a bound toplevel: Hyprland's active
/// window, the toplevel's `activated` state, whether its workspace is in
/// view and, for an accessibility frame, the frame's ACTIVE state.
/// `frame_active` is `None` for a compositor-only window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProofRead {
    pub(crate) compositor_active: bool,
    pub(crate) toplevel_activated: bool,
    pub(crate) in_view: bool,
    pub(crate) frame_active: Option<bool>,
}

impl ProofRead {
    /// Whether every witness reports the window focused. Hyprland keeps a
    /// window active after it moves to a hidden special workspace, so an
    /// active window out of view is not focused.
    pub(crate) fn agrees(self) -> bool {
        self.compositor_active
            && self.toplevel_activated
            && self.in_view
            && self.frame_active != Some(false)
    }
}
