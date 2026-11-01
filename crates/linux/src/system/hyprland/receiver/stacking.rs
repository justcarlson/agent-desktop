/// What decides a client's place in Hyprland's pointer order: whether it
/// floats, is pinned or is fullscreen.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct Stacking {
    pub(crate) floating: bool,
    pub(crate) pinned: bool,
    pub(crate) fullscreen: bool,
}
