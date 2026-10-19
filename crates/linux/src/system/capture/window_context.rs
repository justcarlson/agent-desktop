use agent_desktop_core::{AdapterError, Deadline, DisplayInfo, ImageBuffer, WindowInfo};

use super::window_xid::{XidOutcome, window_xid};
use crate::system::SharedBus;
use crate::system::x11::grab::{Grabber, Raw};
use crate::system::x11::grab_error::GrabError;
use crate::system::x11::stacking_source::StackingSource;

/// The listings a capture re-reads around a grab: the strictly resolved
/// window, the X11 window it maps to, and the displays.
pub(crate) trait WindowLookup {
    fn resolve(&self, window: &WindowInfo, deadline: Deadline) -> Result<WindowInfo, AdapterError>;
    fn xid(&self, window: &WindowInfo, deadline: Deadline) -> XidOutcome;
    fn displays(&self, deadline: Deadline) -> Result<Vec<DisplayInfo>, AdapterError>;
}

pub(crate) struct BusLookup<'a> {
    pub(crate) bus: &'a SharedBus,
}

impl WindowLookup for BusLookup<'_> {
    fn resolve(&self, window: &WindowInfo, deadline: Deadline) -> Result<WindowInfo, AdapterError> {
        crate::system::resolve_window_strict(self.bus, window, deadline)
    }

    fn xid(&self, window: &WindowInfo, deadline: Deadline) -> XidOutcome {
        window_xid(self.bus, window, deadline)
    }

    fn displays(&self, deadline: Deadline) -> Result<Vec<DisplayInfo>, AdapterError> {
        crate::system::list_displays(deadline)
    }
}

pub(crate) type Decode = fn(&Raw) -> Result<Vec<u8>, AdapterError>;
pub(crate) type Encode = fn(&[u8], u32, u32, f64) -> Result<ImageBuffer, AdapterError>;

/// Everything a capture reads from outside the process, behind seams a test
/// replaces.
pub(crate) struct CaptureEnv<'a> {
    pub(crate) grabber: &'a dyn Grabber,
    pub(crate) stacking: &'a dyn StackingSource,
    pub(crate) lookup: &'a dyn WindowLookup,
    pub(crate) decode: Decode,
    pub(crate) encode: Encode,
}

pub(crate) fn real_decode(raw: &Raw) -> Result<Vec<u8>, AdapterError> {
    super::pixels::to_rgb(&raw.format, &raw.data, raw.width, raw.height, raw.stride)
}

/// One window capture in progress: the strictly resolved window, its X11 id,
/// whether the session composites, and the deadline.
pub(crate) struct WindowContext<'a> {
    pub(crate) env: &'a CaptureEnv<'a>,
    pub(crate) window: WindowInfo,
    pub(crate) xid: u32,
    pub(crate) compositing: bool,
    pub(crate) deadline: Deadline,
}

/// The adapter error of a failed grab or drawable read. A window that goes
/// away mid-capture is a stale target, not a missing one.
pub(crate) fn grab_failure(error: GrabError, deadline: Deadline) -> AdapterError {
    match error {
        GrabError::Gone => {
            AdapterError::stale_ref_because("The window closed while it was being captured")
                .with_suggestion("Run 'list-windows' to refresh window identifiers, then retry")
        }
        GrabError::OffScreen => AdapterError::from(GrabError::OffScreen)
            .with_suggestion("Move the window fully onto the screen, then retry")
            .with_details(serde_json::json!({ "reason": "off_screen" })),
        GrabError::Timeout => deadline.timeout_error(),
        other => other.into(),
    }
}
