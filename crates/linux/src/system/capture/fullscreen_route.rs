use agent_desktop_core::{AdapterError, Deadline, ErrorCode};

use super::display::grab_rect;
use super::window_context::{CaptureEnv, grab_failure};
use crate::tree::Extents;

/// The drawable id a grab names to mean the screen's root window.
pub(crate) const ROOT_DRAWABLE: u32 = 0;

/// How a display is captured: from the root window, or not at all when the
/// root window is not a true picture of the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FullRoute {
    Root,
    Refused,
}

/// Under a compositing window manager the root window holds only the
/// manager's own stage, so no X11 read of it is the screen.
pub(crate) fn choose(compositing: bool) -> FullRoute {
    if compositing {
        FullRoute::Refused
    } else {
        FullRoute::Root
    }
}

/// A display's pixels as tightly packed RGB.
pub(crate) struct Painted {
    pub(crate) rgb: Vec<u8>,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

fn refused() -> AdapterError {
    AdapterError::new(
        ErrorCode::PlatformNotSupported,
        "The screen cannot be captured truthfully under a compositing window manager: \
         the root window holds only the manager's own stage, not the application windows",
    )
    .with_suggestion("Capture one window with 'screenshot --window-id'")
    .with_details(serde_json::json!({ "reason": "compositing" }))
}

pub(crate) fn paint(
    env: &CaptureEnv,
    route: FullRoute,
    area: Extents,
    deadline: Deadline,
) -> Result<Painted, AdapterError> {
    match route {
        FullRoute::Root => {
            let raw = env
                .grabber
                .grab(ROOT_DRAWABLE, grab_rect(area)?, deadline)
                .map_err(|error| grab_failure(error, deadline))?;
            Ok(Painted {
                rgb: (env.decode)(&raw)?,
                width: raw.width,
                height: raw.height,
            })
        }
        FullRoute::Refused => Err(refused()),
    }
}

#[cfg(test)]
#[path = "fullscreen_route_tests.rs"]
mod tests;
