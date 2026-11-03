/// One global a compositor advertises in its registry: the numeric name a
/// bind uses, the interface, and the highest version it serves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Global {
    pub(crate) name: u32,
    pub(crate) interface: String,
    pub(crate) version: u32,
}
