/// One compositor toplevel: its `ext_foreign_toplevel_handle_v1`
/// identifier and the Hyprland address the mapping manager gave it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToplevelView {
    pub(crate) identifier: String,
    pub(crate) address: u64,
}
