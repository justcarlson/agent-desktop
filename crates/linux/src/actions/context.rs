use agent_desktop_core::{Deadline, NativeHandle};

use crate::system::SharedBus;
use crate::tree::AtspiElement;

#[expect(dead_code, reason = "no caller yet")]
pub(crate) struct Target<'a> {
    pub(crate) bus: &'a SharedBus,
    pub(crate) handle: &'a NativeHandle,
    pub(crate) element: &'a AtspiElement,
    pub(crate) deadline: Deadline,
}
