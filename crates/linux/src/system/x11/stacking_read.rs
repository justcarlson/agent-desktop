use super::stacking_snapshot::StackingSnapshot;

/// What a stacking read could say. Anything but `Ready` is the absence of
/// evidence.
#[expect(dead_code, reason = "stub the owning lane replaces")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StackingRead {
    Ready(StackingSnapshot),
    NotX11,
    NoManager,
    Unavailable,
}
