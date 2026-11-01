//! Why a cursor shows nothing where it stands: its agent hid it, or its
//! last present aimed at a target on no visible output, which parks it
//! until a present aims at a target that shows.

/// The reasons a cursor is not drawn at its pose.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct CursorConcealment {
    pub(crate) hidden: bool,
    pub(crate) parked: bool,
}

impl CursorConcealment {
    /// Whether any reason keeps the cursor from being drawn.
    pub(crate) fn any(self) -> bool {
        self.hidden || self.parked
    }
}
