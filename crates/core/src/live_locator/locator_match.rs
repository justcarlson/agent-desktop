use super::LocatorMatchData;
use crate::refs::RefEntry;

pub struct LocatorMatch {
    pub data: LocatorMatchData,
    pub document_order: u32,
    pub(crate) entry: RefEntry,
    /// Whether the observation that found this node read every field a ref
    /// and the response need, so reading the node again would only repeat it.
    pub(crate) fully_observed: bool,
}

impl LocatorMatch {
    pub fn into_entry(self) -> RefEntry {
        self.entry
    }
}
