/// The modifiers and layout a Wayland keyboard reports in its `modifiers`
/// event: the depressed, latched and locked modifier masks and the layout
/// index XKB calls the group.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ModifierState {
    pub(crate) depressed: u32,
    pub(crate) latched: u32,
    pub(crate) locked: u32,
    pub(crate) group: u32,
}
