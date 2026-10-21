use super::app_alias::AliasIndex;
use super::desktop_lookup::{read_all, stamps};
use super::desktop_stamp::DesktopStamp;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};
type Stamps = Vec<DesktopStamp>;

/// An index that is rebuilt only when a directory it was read from changes.
pub(crate) struct AliasCache {
    state: Mutex<Option<(Stamps, Arc<AliasIndex>)>>,
}

impl AliasCache {
    pub(crate) const fn new() -> Self {
        Self {
            state: Mutex::new(None),
        }
    }

    pub(crate) fn index(&self, dirs: &[PathBuf]) -> Arc<AliasIndex> {
        let now = stamps(dirs);
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some((seen, index)) = state.as_ref()
            && *seen == now
        {
            return Arc::clone(index);
        }
        let index = Arc::new(AliasIndex::build(&read_all(dirs), dirs));
        *state = Some((now, Arc::clone(&index)));
        index
    }
}
