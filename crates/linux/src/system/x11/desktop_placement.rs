/// Which virtual desktop a window lives on.
#[expect(dead_code, reason = "stub the owning lane replaces")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DesktopPlacement {
    Sticky,
    On(u32),
    Unknown,
}
