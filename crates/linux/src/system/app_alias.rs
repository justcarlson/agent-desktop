use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::SystemTime;

use super::desktop_entry::parse;
use super::desktop_exec::program_word;
use super::desktop_lookup::{current_dirs, read_all, stamps};
use super::launch_attach::KEY_TABLE;

struct IndexedEntry {
    id: String,
    name: String,
    programs: Vec<String>,
    id_programs: Vec<String>,
}

/// The desktop entries of the machine, as the names each executable goes by.
pub(crate) struct AliasIndex {
    entries: Vec<IndexedEntry>,
}

pub(super) fn basename_lower(program: &str) -> Option<String> {
    Path::new(program)
        .file_name()
        .map(|name| name.to_string_lossy().to_lowercase())
        .filter(|name| !name.is_empty())
}

impl AliasIndex {
    fn build(files: &[(String, String)]) -> Self {
        let entries = files
            .iter()
            .filter_map(|(id, text)| parse(id, text))
            .filter(|entry| entry.traits.application && !entry.traits.hidden)
            .map(|entry| {
                let mut id_programs: Vec<String> = entry
                    .exec
                    .as_deref()
                    .and_then(program_word)
                    .into_iter()
                    .filter_map(|program| basename_lower(&program))
                    .collect();
                let mut programs = id_programs.clone();
                programs.extend(entry.try_exec.as_deref().and_then(basename_lower));
                let id = entry.id.to_lowercase();
                for (key, extra) in KEY_TABLE {
                    if *key == id || id_programs.iter().any(|program| program == key) {
                        id_programs.extend(extra.iter().map(|name| (*name).to_owned()));
                    }
                    if *key == id || programs.iter().any(|program| program == key) {
                        programs.extend(extra.iter().map(|name| (*name).to_owned()));
                    }
                }
                programs.dedup();
                IndexedEntry {
                    id: entry.id,
                    name: entry.name,
                    programs,
                    id_programs,
                }
            })
            .collect();
        Self { entries }
    }

    /// The `Name` of every entry whose `Exec` program or `TryExec` has this
    /// executable basename, compared exactly and ignoring case.
    pub(crate) fn names_for(&self, exe_basename: &str) -> Vec<String> {
        let wanted = exe_basename.to_lowercase();
        let mut names: Vec<String> = Vec::new();
        for entry in &self.entries {
            if entry.programs.contains(&wanted) && !names.contains(&entry.name) {
                names.push(entry.name.clone());
            }
        }
        names
    }

    /// The id uniquely associated with this executable through `Exec` or a
    /// known launcher/server mapping. `TryExec` only checks availability and
    /// cannot identify the running application. Shared executable basenames
    /// cannot distinguish desktop entries, so they receive no id alias.
    pub(crate) fn ids_for(&self, exe_basename: &str) -> Vec<String> {
        let wanted = exe_basename.to_lowercase();
        let mut entries = self
            .entries
            .iter()
            .filter(|entry| entry.id_programs.contains(&wanted));
        match (entries.next(), entries.next()) {
            (Some(entry), None) => vec![entry.id.clone()],
            _ => Vec::new(),
        }
    }

    /// The id of every entry whose `Name` equals `name`, ignoring case.
    pub(crate) fn ids_named(&self, name: &str) -> Vec<String> {
        self.entries
            .iter()
            .filter(|entry| entry.name.eq_ignore_ascii_case(name))
            .map(|entry| entry.id.clone())
            .collect()
    }
}

type Stamps = Vec<Option<SystemTime>>;

/// An index that is rebuilt only when a directory it was read from changes.
pub(crate) struct AliasCache {
    state: Mutex<Option<(Stamps, Arc<AliasIndex>)>>,
    builds: AtomicUsize,
}

impl AliasCache {
    pub(crate) const fn new() -> Self {
        Self {
            state: Mutex::new(None),
            builds: AtomicUsize::new(0),
        }
    }

    pub(crate) fn index(
        &self,
        dirs: &[PathBuf],
        read: impl Fn(&[PathBuf]) -> Vec<(String, String)>,
    ) -> Arc<AliasIndex> {
        let now = stamps(dirs);
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some((seen, index)) = state.as_ref()
            && *seen == now
        {
            return Arc::clone(index);
        }
        self.builds.fetch_add(1, Ordering::Relaxed);
        let index = Arc::new(AliasIndex::build(&read(dirs)));
        *state = Some((now, Arc::clone(&index)));
        index
    }

    #[cfg(test)]
    pub(crate) fn builds(&self) -> usize {
        self.builds.load(Ordering::Relaxed)
    }
}

static CACHE: AliasCache = AliasCache::new();

/// The index of the entries the process environment names.
pub(crate) fn current_index() -> Arc<AliasIndex> {
    CACHE.index(&current_dirs(), read_all)
}

#[cfg(test)]
#[path = "app_alias_tests.rs"]
mod tests;
