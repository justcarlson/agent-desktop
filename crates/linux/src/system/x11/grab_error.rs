#![expect(dead_code, reason = "stub the owning lane replaces")]

use super::MonitorFault;

pub(crate) enum GrabError {
    Gone,
    Unviewable,
    OffScreen,
    Resource,
    Fault(MonitorFault),
    Timeout,
}
