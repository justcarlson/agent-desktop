/// What a keyboard delivery holds down between keysym events: how many Shift
/// keysyms the caller pressed, and the evdev codes this backend wrapped in a
/// Shift press of its own.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct PressState {
    pub(crate) explicit_shift: u32,
    pub(crate) wrapped: Vec<u32>,
}
