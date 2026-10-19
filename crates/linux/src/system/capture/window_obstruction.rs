#![expect(dead_code, reason = "stub the owning lane replaces")]

use crate::system::x11::stacking_read::StackingRead;
use crate::tree::Extents;

pub(crate) enum Cover {
    Clear,
    Covered,
    Unknown,
}

pub(crate) fn cover(_read: &StackingRead, _target_client: u32, _rect: Extents) -> Cover {
    Cover::Unknown
}
