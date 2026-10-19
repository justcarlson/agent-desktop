#![expect(dead_code, reason = "stub the owning lane replaces")]

use agent_desktop_core::Deadline;

use super::MonitorFault;
use super::drawable_info::DrawableInfo;
use super::grab_error::GrabError;
use super::grab_rect::GrabRect;
use crate::system::capture::pixels::VisualFormat;

pub(crate) struct Raw {
    pub(crate) data: Vec<u8>,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) stride: usize,
    pub(crate) format: VisualFormat,
}

pub(crate) trait Grabber: Send + Sync {
    fn describe(&self, window: u32, deadline: Deadline) -> Result<DrawableInfo, GrabError>;
    fn grab(&self, drawable: u32, rect: GrabRect, deadline: Deadline) -> Result<Raw, GrabError>;
    fn compositing(&self, deadline: Deadline) -> Result<bool, GrabError>;
}

pub(crate) struct DisplayGrabber;

impl Grabber for DisplayGrabber {
    fn describe(&self, _window: u32, _deadline: Deadline) -> Result<DrawableInfo, GrabError> {
        Err(GrabError::Fault(MonitorFault::Unreadable))
    }

    fn grab(&self, _drawable: u32, _rect: GrabRect, _deadline: Deadline) -> Result<Raw, GrabError> {
        Err(GrabError::Fault(MonitorFault::Unreadable))
    }

    fn compositing(&self, _deadline: Deadline) -> Result<bool, GrabError> {
        Err(GrabError::Fault(MonitorFault::Unreadable))
    }
}
