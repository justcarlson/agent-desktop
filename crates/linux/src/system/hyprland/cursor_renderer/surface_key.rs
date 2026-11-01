//! Which layer surface a drawing goes to. Each agent cursor owns its own
//! surfaces, named by the cursor's key; the output part is the output a
//! placement pins the surface to, so a cursor can hold one surface per
//! output it touches.

use super::placement::Placement;

/// One cursor surface: its cursor and the output it is pinned to, if any.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct SurfaceKey {
    pub(crate) cursor: String,
    pub(crate) output: Option<u32>,
}

impl SurfaceKey {
    /// The surface of `cursor` that `placement` draws on.
    pub(crate) fn placed(cursor: &str, placement: &Placement) -> Self {
        Self {
            cursor: cursor.to_owned(),
            output: placement.output,
        }
    }
}
