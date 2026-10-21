use x11rb::protocol::xproto::Window;

use super::overlays::Overlays;

pub(super) struct StackSelection {
    pub(super) ids: Vec<Window>,
    pub(super) overlays: Overlays,
}
