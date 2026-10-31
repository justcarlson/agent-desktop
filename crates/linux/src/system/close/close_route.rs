use std::cell::RefCell;

use agent_desktop_core::{Deadline, ErrorCode};

use crate::system::hyprland::bind::routed_binding::routed_binding;
use crate::system::hyprland::close::compositor_close::CompositorClose;

/// Where close requests for one process go. A bound Hyprland instance that
/// advertises the wlr foreign toplevel manager takes them through the
/// compositor; any other session keeps the X11 route, which on a Wayland
/// session reaches XWayland windows only. `Unreachable` is a bound
/// compositor whose toplevel session could not be opened.
pub(super) enum CloseRoute {
    X11,
    Compositor(Box<RefCell<CompositorClose>>),
    Unreachable,
}

impl CloseRoute {
    pub(super) fn open(pid: u32, deadline: Deadline) -> Self {
        let Ok(Some(binding)) = routed_binding() else {
            return Self::X11;
        };
        match CompositorClose::open(binding, pid, deadline) {
            Ok(close) => Self::Compositor(Box::new(RefCell::new(close))),
            Err(error) if error.code == ErrorCode::PlatformNotSupported => Self::X11,
            Err(_) => Self::Unreachable,
        }
    }
}
