/// One live wlr foreign toplevel of the bound compositor: its handle's
/// protocol id on the session's connection, the Hyprland address it maps
/// to, its title, whether it is the activated toplevel, and whether it is
/// the child of another toplevel, as a dialog is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WlrWindow {
    pub(crate) id: u32,
    pub(crate) address: u64,
    pub(crate) title: String,
    pub(crate) activated: bool,
    pub(crate) child: bool,
}
