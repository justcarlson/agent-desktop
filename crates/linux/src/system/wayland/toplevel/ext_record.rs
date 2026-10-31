/// What an `ext_foreign_toplevel_handle_v1` told this connection: its
/// opaque identifier, the Hyprland address the mapping manager answered,
/// and whether the mapping manager answered at all.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ExtRecord {
    pub(crate) identifier: String,
    pub(crate) address: Option<u64>,
    pub(crate) answered: bool,
}
