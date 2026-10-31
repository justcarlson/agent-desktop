/// What a `zwlr_foreign_toplevel_handle_v1` told this connection: the
/// Hyprland address the mapping manager answered, whether its last state
/// array holds `activated`, and whether the toplevel closed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct WlrRecord {
    pub(crate) address: Option<u64>,
    pub(crate) answered: bool,
    pub(crate) activated: bool,
    pub(crate) closed: bool,
}
