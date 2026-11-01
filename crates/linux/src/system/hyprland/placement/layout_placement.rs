use agent_desktop_core::Rect;

/// An element's rectangle in global layout logical pixels, the output its
/// centre lies on, and whether the compositor shows it there now.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Placement {
    pub(crate) rect: Rect,
    pub(crate) output: Option<String>,
    pub(crate) shown: bool,
}
