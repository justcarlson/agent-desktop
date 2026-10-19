#![expect(dead_code, reason = "stub the owning lane replaces")]

pub(crate) enum MapState {
    Viewable,
    Unviewable,
    Unmapped,
}
