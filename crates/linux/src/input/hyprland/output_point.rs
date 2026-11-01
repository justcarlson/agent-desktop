/// A pointer position for a virtual pointer bound to `output`: the point
/// and the output's logical size, both multiplied by 120 so a fractional
/// logical position keeps its precision, and the same point in the global
/// layout in logical pixels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OutputPoint {
    pub(crate) output: String,
    pub(crate) x: u32,
    pub(crate) y: u32,
    pub(crate) x_extent: u32,
    pub(crate) y_extent: u32,
    pub(crate) global: (i32, i32),
}
