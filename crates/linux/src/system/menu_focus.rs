use agent_desktop_core::{Deadline, WindowInfo};

use super::{SharedBus, menu_surface};
use crate::tree::AtspiElement;

/// Whether the window's application has an open menu. Only asked when no
/// window claims focus: an open menu holds the keyboard grab for the
/// application that owns it, so its presence is that application's focus.
pub(crate) fn open_for_window(bus: &SharedBus, window: &WindowInfo, deadline: Deadline) -> bool {
    let Ok((bus_name, path)) = super::window_id::decode(&window.id) else {
        return false;
    };
    open_at(
        bus,
        &AtspiElement::new(bus_name, path, window.pid.get()),
        deadline,
    )
}

pub(crate) fn open_at(bus: &SharedBus, window: &AtspiElement, deadline: Deadline) -> bool {
    menu_surface::popups_at(bus, window, false, deadline).is_ok_and(|popups| !popups.is_empty())
}
