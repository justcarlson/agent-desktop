/// What decides a client's place in Hyprland's pointer order: whether it
/// floats, is pinned or is fullscreen, and whether it may stay above a
/// fullscreen window. `over_fullscreen` is `None` when Hyprland does not
/// report it.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct Stacking {
    pub(crate) floating: bool,
    pub(crate) pinned: bool,
    pub(crate) fullscreen: bool,
    pub(crate) over_fullscreen: Option<bool>,
}
