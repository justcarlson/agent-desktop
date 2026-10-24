use super::app_alias::AliasIndex;
use crate::system::launch::desktop_lookup::{read_all, stamps};
use crate::system::launch::desktop_stamp::DesktopStamp;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, SystemTime};
type Stamps = Vec<DesktopStamp>;

/// An index that is rebuilt only when a directory it was read from changes.
pub(crate) struct AliasCache {
    state: Mutex<Option<(Stamps, SystemTime, Arc<AliasIndex>)>>,
}

impl AliasCache {
    pub(crate) const fn new() -> Self {
        Self {
            state: Mutex::new(None),
        }
    }

    pub(crate) fn index(&self, dirs: &[PathBuf]) -> Arc<AliasIndex> {
        let built_at = SystemTime::now();
        let now = stamps(dirs);
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some((seen, built_at, index)) = state.as_ref()
            && *seen == now
            && built_at
                .checked_sub(Duration::from_millis(100))
                .is_some_and(|cutoff| now.iter().all(|stamp| stamp.is_older_than(cutoff)))
        {
            return Arc::clone(index);
        }
        let index = Arc::new(AliasIndex::build(&read_all(dirs), dirs));
        *state = Some((now, built_at, Arc::clone(&index)));
        index
    }
}

#[cfg(test)]
#[path = "alias_cache_tests.rs"]
mod tests;
