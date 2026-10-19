/// What a window manager's window type says a window is for.
#[expect(dead_code, reason = "stub the owning lane replaces")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WindowKind {
    Normal,
    Dialog,
    Utility,
    Menu,
    Dock,
    Desktop,
    Other,
}
