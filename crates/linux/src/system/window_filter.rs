use crate::tree::role_numbers::{DIALOG, FILLER, FRAME, PANEL, WINDOW};
use crate::tree::state_bits::{ACTIVE, ICONIFIED, MODAL, SHOWING, StateBits, VISIBLE};
use crate::tree::{COMPONENT_INTERFACE, Field};

const HELPER_PROCESSES: [&str; 2] = ["gnome-shell", "mutter-x11-frames"];
const BUBBLE_CLASS: &str = "RootView";
const MENU_HOST_CLASS: &str = "MenuHostRootView";

/// Whether a child of an application root is a top-level window: a frame,
/// dialog or window role that implements the component interface, or a panel
/// that carries `MODAL` (GTK 4 presents its shortcuts window that way, and an
/// application root has no panel child otherwise), or a filler that is
/// `SHOWING` and `VISIBLE` (Qt presents a top-level `QWidget` that is not a
/// `QMainWindow` that way, L30-28). GTK 3 names the frame `frame` and
/// GTK 4 names the same numeric role `application`, so only the number is
/// read.
pub(crate) fn is_window_shape(
    role: &Field<u32>,
    interfaces: &Field<Vec<String>>,
    states: &Field<Vec<u32>>,
) -> bool {
    let shaped = match role.present() {
        Some(&(FRAME | DIALOG | WINDOW)) => true,
        Some(&PANEL) => states
            .present()
            .is_some_and(|words| StateBits::new(words).has(MODAL)),
        Some(&FILLER) => states.present().is_some_and(|words| {
            let bits = StateBits::new(words);
            bits.has(SHOWING) && bits.has(VISIBLE)
        }),
        _ => false,
    };
    shaped
        && interfaces
            .present()
            .is_some_and(|list| list.iter().any(|name| name == COMPONENT_INTERFACE))
}

/// Whether an application is one of the desktop's own helpers, whose frames
/// are decorations and the shell's overview and never a user's window. The
/// registry name is compared with the executable name because `comm` is cut
/// at 15 characters.
pub(crate) fn is_helper_app(root_name: &str, exe_basename: Option<&str>) -> bool {
    [Some(root_name), exe_basename]
        .into_iter()
        .flatten()
        .any(|name| {
            HELPER_PROCESSES
                .iter()
                .any(|helper| name.eq_ignore_ascii_case(helper))
        })
}

/// Whether a Chromium frame is a bubble or popup rather than a browser
/// window: its view class is `RootView` and it has no title, where a window
/// is a `BrowserRootView`.
pub(crate) fn is_bubble(class: Option<&str>, title: &str) -> bool {
    is_nameless(class, BUBBLE_CLASS, title)
}

fn is_nameless(class: Option<&str>, wanted: &str, title: &str) -> bool {
    class == Some(wanted) && title.trim().is_empty()
}

/// Whether a Chromium frame is the host of an open menu: a context or main
/// menu opens as a nameless frame of class `MenuHostRootView` under the
/// application root, carrying `ACTIVE` while it is open. It is a popup
/// of the window that opened it, never a window.
pub(crate) fn is_menu_popup_frame(class: Option<&str>, title: &str) -> bool {
    is_nameless(class, MENU_HOST_CLASS, title)
}

/// What a frame's state set says. `None` is a state read that failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FrameState {
    pub(crate) visible: Option<bool>,
    pub(crate) minimized: Option<bool>,
    pub(crate) active: Option<bool>,
    pub(crate) modal: Option<bool>,
}

pub(crate) fn frame_state(states: &Field<Vec<u32>>) -> FrameState {
    let bits = states.present().map(|words| StateBits::new(words));
    FrameState {
        visible: bits.map(|bits| bits.has(SHOWING)),
        minimized: bits.map(|bits| bits.has(ICONIFIED)),
        active: bits.map(|bits| bits.has(ACTIVE)),
        modal: bits.map(|bits| bits.has(MODAL)),
    }
}

#[cfg(test)]
#[path = "window_filter_tests.rs"]
mod tests;
