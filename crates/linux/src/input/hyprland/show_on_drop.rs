use crate::system::hyprland::cursor_hider::CursorHider;

/// Shows what a successful `hide` hid when it goes out of scope, so every
/// exit path after the hide shows again.
pub(crate) struct ShowOnDrop<'a> {
    pub(crate) hider: &'a mut dyn CursorHider,
}

impl<'a> ShowOnDrop<'a> {
    pub(crate) fn new(hider: &'a mut dyn CursorHider) -> Self {
        Self { hider }
    }
}

impl Drop for ShowOnDrop<'_> {
    fn drop(&mut self) {
        self.hider.show();
    }
}
