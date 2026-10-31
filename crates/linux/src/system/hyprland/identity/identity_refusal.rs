/// Why a window could not be bound to exactly one compositor toplevel.
/// Every refusal is decided before anything is sent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum IdentityRefusal {
    ProcessChanged,
    OtherCompositor,
    ToplevelGone,
    NoCandidate,
    Ambiguous(Vec<(u64, String)>),
}
