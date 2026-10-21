/// What the window's own frame says about itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct FrameView {
    pub(super) minimized: Option<bool>,
    pub(super) active: Option<bool>,
    pub(super) modal: Option<bool>,
    pub(super) dialog: bool,
}
