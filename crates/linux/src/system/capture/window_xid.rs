#![expect(dead_code, reason = "stub the owning lane replaces")]

use agent_desktop_core::{Deadline, WindowInfo};

use crate::system::SharedBus;

pub(crate) enum XidOutcome {
    One(u32),
    Several(Vec<u32>),
    NoClient,
    NoManager,
    Unavailable,
}

pub(crate) fn window_xid(
    _bus: &SharedBus,
    _window: &WindowInfo,
    _deadline: Deadline,
) -> XidOutcome {
    XidOutcome::Unavailable
}
