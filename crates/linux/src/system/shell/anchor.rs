use super::container::Landmark;

/// The node a search anchors on. A landmark's container is found a fixed
/// number of levels above its anchor, so a container that holds nothing
/// readable while closed is still found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Anchor {
    Landmark(Landmark),
    SystemButton,
}
