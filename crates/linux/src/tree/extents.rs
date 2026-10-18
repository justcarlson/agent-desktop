/// Screen-space extents as the toolkit reports them. Zero and negative
/// values are reported values, not errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Extents {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) width: i32,
    pub(crate) height: i32,
}
