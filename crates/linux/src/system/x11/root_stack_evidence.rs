use x11rb::protocol::xproto::Window;

pub(super) struct RootStackEvidence {
    pub(super) viewable: bool,
    pub(super) override_redirect: bool,
    pub(super) siblings: Vec<Window>,
    pub(super) managed_frames: Vec<(Window, Window)>,
}
