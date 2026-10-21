use crate::system::alias_cache::AliasCache;
use std::path::Path;
use std::sync::Arc;

use super::desktop_entry::parse;
use super::desktop_exec::program_word;
use super::desktop_lookup::current_dirs;
use super::launch_attach::KEY_TABLE;

struct IndexedEntry {
    id: String,
    name: String,
    programs: Vec<String>,
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
    pub(super) fn build(files: &[(String, String)]) -> Self {
        let entries = files
            .iter()
            .filter_map(|(id, text)| parse(id, text))
            .filter(|entry| entry.traits.application && !entry.traits.hidden)
            .map(|entry| {
                let mut programs: Vec<String> = entry
                    .exec
                    .as_deref()
                    .and_then(program_word)
                    .into_iter()
                    .filter_map(|program| basename_lower(&program))
                    .collect();
                let id = entry.id.to_lowercase();
                for (key, extra) in KEY_TABLE {
                    if *key == id || programs.iter().any(|program| program == key) {
                        programs.extend(extra.iter().map(|name| (*name).to_owned()));
                    }
                }
                programs.dedup();
                IndexedEntry {
                    id: entry.id,
                    name: entry.name,
                    programs,
                }
            })
            .collect();
        Self { entries }
    }

    /// The name uniquely associated with this executable through `Exec` or a
    /// known launcher/server mapping.
    pub(crate) fn names_for(&self, exe_basename: &str) -> Vec<String> {
        let wanted = exe_basename.to_lowercase();
        match self.unique_entry(&wanted) {
            Some(entry) => vec![entry.name.clone()],
            None => Vec::new(),
        }
    }

    /// The id uniquely associated with this executable through `Exec` or a
    /// known launcher/server mapping. `TryExec` only checks availability and
    /// cannot identify the running application. Shared executable basenames
    /// cannot distinguish desktop entries, so they receive no id alias.
    pub(crate) fn ids_for(&self, exe_basename: &str) -> Vec<String> {
        let wanted = exe_basename.to_lowercase();
        match self.unique_entry(&wanted) {
            Some(entry) => vec![entry.id.clone()],
            None => Vec::new(),
        }
    }

    fn unique_entry(&self, wanted: &str) -> Option<&IndexedEntry> {
        let mut entries = self
            .entries
            .iter()
            .filter(|entry| entry.programs.iter().any(|program| program == wanted));
        match (entries.next(), entries.next()) {
            (Some(entry), None) => Some(entry),
            _ => None,
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

static CACHE: AliasCache = AliasCache::new();

/// The index of the entries the process environment names.
pub(crate) fn current_index() -> Arc<AliasIndex> {
    CACHE.index(&current_dirs())
}

#[cfg(test)]
#[path = "app_alias_tests.rs"]
mod tests;
