use crate::tree::role_numbers::{DIALOG, FRAME, PANEL, WINDOW};
use crate::tree::state_bits::{ACTIVE, ICONIFIED, MODAL, SHOWING, StateBits};
use crate::tree::{COMPONENT_INTERFACE, Field};

const HELPER_PROCESSES: [&str; 2] = ["gnome-shell", "mutter-x11-frames"];
const BUBBLE_CLASS: &str = "RootView";

/// Whether a child of an application root is a top-level window: a frame,
/// dialog or window role that implements the component interface, or a panel
/// that carries `MODAL` (GTK 4 presents its shortcuts window that way, and an
/// application root has no panel child otherwise). GTK 3 names the frame `frame` and
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
    class == Some(BUBBLE_CLASS) && title.trim().is_empty()
}

/// What a frame's state set says. `None` is a state read that failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FrameState {
    pub(crate) visible: Option<bool>,
    pub(crate) minimized: Option<bool>,
    pub(crate) active: Option<bool>,
}

pub(crate) fn frame_state(states: &Field<Vec<u32>>) -> FrameState {
    match states.present() {
        Some(words) => {
            let bits = StateBits::new(words);
            FrameState {
                visible: Some(bits.has(SHOWING)),
                minimized: Some(bits.has(ICONIFIED)),
                active: Some(bits.has(ACTIVE)),
            }
        }
        None => FrameState {
            visible: None,
            minimized: None,
            active: None,
        },
    }
}

#[cfg(test)]
#[path = "window_filter_tests.rs"]
mod tests;
