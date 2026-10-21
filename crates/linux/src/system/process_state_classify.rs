/// What the accessibility probe of a process answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Probe {
    Answered,
    TimedOut,
    Unavailable,
}
