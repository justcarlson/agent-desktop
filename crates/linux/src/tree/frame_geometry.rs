use super::Extents;
use crate::system::toolkit_id::ToolkitId;

/// The rectangle a pointer can be aimed at, in screen coordinates: the
/// toolkit's own for global-position toolkits, the window rectangle moved to
/// the client origin otherwise, and `None` when neither can be proved.
#[expect(dead_code, reason = "stub the owning lane replaces")]
pub(crate) fn pointer_bounds(
    _toolkit: &ToolkitId,
    _screen: Extents,
    _window: Extents,
    _client_origin: Option<(i32, i32)>,
) -> Option<Extents> {
    None
}

/// Whether the screen rectangle and the window rectangle describe the same
/// place once the window rectangle is moved to the client origin.
#[expect(dead_code, reason = "stub the owning lane replaces")]
pub(crate) fn origins_agree(
    _screen: Extents,
    _window: Extents,
    _client_origin: (i32, i32),
) -> bool {
    false
}
