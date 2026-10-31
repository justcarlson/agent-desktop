/// The foreign toplevel handle a mapping request asked about: the index of
/// an `ext` handle or of a wlr handle, in announce order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MappingTarget {
    Ext(usize),
    Wlr(usize),
}
