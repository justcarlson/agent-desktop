/// One compositor toplevel as both foreign toplevel protocols name it: the
/// ext identifier, the Hyprland address both handles map to, whether the
/// wlr handle reports it activated, and the index of the wlr handle that
/// can activate it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToplevelView {
    pub(crate) identifier: String,
    pub(crate) address: u64,
    pub(crate) activated: bool,
    pub(crate) wlr_index: Option<usize>,
}
