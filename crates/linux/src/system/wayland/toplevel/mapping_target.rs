/// The handle a mapping request asked about, by its index in the session's
/// ext or wlr list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MappingTarget {
    Ext(usize),
    Wlr(usize),
}
