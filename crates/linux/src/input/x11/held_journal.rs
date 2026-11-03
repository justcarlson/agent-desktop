use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

use agent_desktop_core::{AdapterError, DeliverySemantics, ErrorCode};
use serde_json::{Map, Value, json};

use super::sink::X11Sink;
use super::sink_error::SinkError;
use crate::input::held_input::HeldInput;
use crate::input::keyboard::keymap_edit::KeymapEdit;
use crate::system::process::{liveness, proc_stat, process_facts};

const JOURNAL_DIR: &str = "agent-desktop-input";
const RECORD_EXTENSION: &str = "json";
const QUARANTINE_EXTENSION: &str = "unreadable";

#[derive(Debug, Clone)]
pub(crate) struct HeldJournal {
    dir: PathBuf,
    display: String,
    pid: u32,
    instance: String,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct StaleEntry {
    path: PathBuf,
    pub(crate) keys: Vec<u8>,
    pub(crate) buttons: Vec<u8>,
    pub(crate) remaps: Vec<KeymapEdit>,
}

fn codes(value: &Value, field: &str) -> Vec<u8> {
    value[field]
        .as_array()
        .map(|list| {
            list.iter()
                .filter_map(|code| code.as_u64().and_then(|code| u8::try_from(code).ok()))
                .collect()
        })
        .unwrap_or_default()
}

enum Record {
    Own(StaleEntry),
    Entry(StaleEntry),
    Unreadable(PathBuf),
}

/// A journal file on this display that the caller may recover: this
/// process's own record, a dead process's record, or one that cannot be read.
pub(crate) enum JournalFile {
    Own(PathBuf, Value),
    Stale(PathBuf, Value),
    Unreadable(PathBuf),
}

fn stale_entry(path: PathBuf, value: &Value) -> StaleEntry {
    StaleEntry {
        path,
        keys: codes(value, "keys"),
        buttons: codes(value, "buttons"),
        remaps: remaps(value),
    }
}

fn remaps(value: &Value) -> Vec<KeymapEdit> {
    value["remaps"]
        .as_array()
        .map(|list| {
            list.iter()
                .filter_map(|edit| {
                    let keycode = u8::try_from(edit["keycode"].as_u64()?).ok()?;
                    let keysyms = edit["keysyms"]
                        .as_array()?
                        .iter()
                        .map(|keysym| u32::try_from(keysym.as_u64()?).ok())
                        .collect::<Option<Vec<u32>>>()?;
                    Some(KeymapEdit { keycode, keysyms })
                })
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn process_is_alive(pid: u32, instance: &str) -> bool {
    if instance.is_empty() {
        return Path::new(&format!("/proc/{pid}")).exists();
    }
    matches!(
        liveness::identity(pid, instance),
        crate::system::process::identity::Identity::Same
            | crate::system::process::identity::Identity::Unreadable
    )
}

/// The folder that holds every record of held input for this user session.
pub(crate) fn runtime_journal_dir() -> Option<PathBuf> {
    let runtime = std::env::var_os("XDG_RUNTIME_DIR").filter(|dir| !dir.is_empty())?;
    Some(journal_dir_in(Path::new(&runtime)))
}

/// The folder of held-input records under the runtime directory `runtime`.
pub(crate) fn journal_dir_in(runtime: &Path) -> PathBuf {
    runtime.join(JOURNAL_DIR)
}

impl HeldJournal {
    pub(crate) fn in_runtime_dir(display: &str) -> Option<Self> {
        Some(Self::at(
            runtime_journal_dir()?,
            display,
            std::process::id(),
        ))
    }

    pub(crate) fn at(dir: PathBuf, display: &str, pid: u32) -> Self {
        Self {
            dir,
            display: display.to_owned(),
            pid,
            instance: proc_stat::read_stat(std::process::id())
                .ok()
                .flatten()
                .map(|stat| process_facts::instance_token(stat.start_time))
                .unwrap_or_default(),
        }
    }

    fn own_path(&self) -> PathBuf {
        self.dir.join(format!("{}.json", self.pid))
    }

    pub(crate) fn record(&self, held: &HeldInput, remaps: &[KeymapEdit]) -> std::io::Result<()> {
        if held.keys.is_empty() && held.buttons.is_empty() && remaps.is_empty() {
            self.clear();
            return Ok(());
        }
        let mut fields = Map::new();
        fields.insert("keys".to_owned(), json!(held.keys));
        fields.insert("buttons".to_owned(), json!(held.buttons));
        fields.insert(
            "remaps".to_owned(),
            remaps
                .iter()
                .map(|edit| json!({ "keycode": edit.keycode, "keysyms": edit.keysyms }))
                .collect(),
        );
        self.write(fields)
    }

    /// Replaces this process's record with `entry`, stamped with the display
    /// and the process identity.
    pub(crate) fn write(&self, mut entry: Map<String, Value>) -> std::io::Result<()> {
        if self.instance.is_empty() {
            return Err(std::io::Error::other(
                "The input process identity could not be read",
            ));
        }
        entry.insert("display".to_owned(), json!(self.display));
        entry.insert("pid".to_owned(), json!(self.pid));
        entry.insert("instance".to_owned(), json!(self.instance));
        let entry = Value::Object(entry);
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&self.dir)?;
        let pending = self.dir.join(format!("{}.json.tmp", self.pid));
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&pending)?;
        file.write_all(entry.to_string().as_bytes())?;
        drop(file);
        std::fs::rename(&pending, self.own_path())
    }

    pub(crate) fn clear(&self) {
        let _ = std::fs::remove_file(self.own_path());
    }

    fn classify(&self, path: PathBuf, alive: &dyn Fn(u32, &str) -> bool) -> Option<JournalFile> {
        if path.extension()?.to_str()? != RECORD_EXTENSION {
            return None;
        }
        let pid: u32 = path.file_stem()?.to_str()?.parse().ok()?;
        let parsed: Option<Value> = std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok());
        let instance = parsed
            .as_ref()
            .and_then(|value| value["instance"].as_str())
            .unwrap_or_default();
        if parsed
            .as_ref()
            .is_some_and(|value| value["display"].is_string())
            && instance.is_empty()
        {
            return Some(JournalFile::Unreadable(path));
        }
        let own = pid == self.pid && !instance.is_empty() && instance == self.instance;
        if !own && alive(pid, instance) {
            return None;
        }
        let Some(value) = parsed.filter(|value| value["display"].is_string()) else {
            return Some(JournalFile::Unreadable(path));
        };
        if value["display"].as_str() != Some(self.display.as_str()) {
            return None;
        }
        Some(if own {
            JournalFile::Own(path, value)
        } else {
            JournalFile::Stale(path, value)
        })
    }

    /// The records of this display that a dead process or this process left,
    /// and every record that cannot be read.
    pub(crate) fn files(&self, alive: &dyn Fn(u32, &str) -> bool) -> Vec<JournalFile> {
        let Ok(listing) = std::fs::read_dir(&self.dir) else {
            return Vec::new();
        };
        listing
            .flatten()
            .filter_map(|file| self.classify(file.path(), alive))
            .collect()
    }

    fn scan(&self, alive: &dyn Fn(u32, &str) -> bool) -> Vec<Record> {
        self.files(alive)
            .into_iter()
            .map(|file| match file {
                JournalFile::Own(path, value) => Record::Own(stale_entry(path, &value)),
                JournalFile::Stale(path, value) => Record::Entry(stale_entry(path, &value)),
                JournalFile::Unreadable(path) => Record::Unreadable(path),
            })
            .collect()
    }

    pub(crate) fn stale_entries(&self, alive: &dyn Fn(u32, &str) -> bool) -> Vec<StaleEntry> {
        self.scan(alive)
            .into_iter()
            .filter_map(|record| match record {
                Record::Entry(entry) => Some(entry),
                _ => None,
            })
            .collect()
    }

    pub(crate) fn has_stale_entries(&self) -> bool {
        !self.stale_entries(&process_is_alive).is_empty()
    }

    pub(crate) fn release_readable_stale(&self, sink: &mut dyn X11Sink) {
        for record in self.scan(&process_is_alive) {
            if let Record::Entry(entry) = record {
                let _ = release_entry(sink, &entry);
            }
        }
    }

    /// Recovers pending input, including this process's previous call, while owning the worker slot.
    pub(crate) fn release_stale(&self, sink: &mut dyn X11Sink) -> Result<(), AdapterError> {
        self.release_stale_where(sink, &process_is_alive)
    }

    fn release_stale_where(
        &self,
        sink: &mut dyn X11Sink,
        alive: &dyn Fn(u32, &str) -> bool,
    ) -> Result<(), AdapterError> {
        let mut unreadable = 0_usize;
        let mut recovered = Ok(());
        for record in self.scan(alive) {
            match record {
                Record::Entry(entry) | Record::Own(entry) => {
                    recovered = recovered.and(release_entry(sink, &entry));
                }
                Record::Unreadable(path) => {
                    quarantine(&path);
                    unreadable += 1;
                }
            }
        }
        if unreadable == 0 {
            return recovered.map_err(|error| recovery_refusal(error.into_adapter()));
        }
        Err(unreadable_record_refusal(unreadable))
    }
}

fn release_entry(sink: &mut dyn X11Sink, entry: &StaleEntry) -> Result<(), SinkError> {
    let mut recovered = Ok(());
    for code in &entry.buttons {
        recovered = recovered.and(sink.button(*code, false));
    }
    for code in &entry.keys {
        recovered = recovered.and(sink.key(*code, false));
    }
    for edit in &entry.remaps {
        recovered = recovered.and(sink.edit_keymap(edit));
    }
    recovered.and(sink.round_trip())?;
    std::fs::remove_file(&entry.path).map_err(|error| SinkError::Journal(error.to_string()))
}

/// `error`, from releasing held input a dead call left, as the refusal of the
/// call that tried.
pub(crate) fn recovery_refusal(error: AdapterError) -> AdapterError {
    error
        .with_suggestion(
            "Check the held input and retry only after the display connection recovers",
        )
        .with_details(json!({
            "cause": "held_input_recovery_failed",
            "physical_delivery_started": false,
        }))
        .with_disposition(DeliverySemantics::not_delivered())
}

pub(crate) fn quarantine(path: &Path) {
    if std::fs::rename(path, path.with_extension(QUARANTINE_EXTENSION)).is_err() {
        let _ = std::fs::remove_file(path);
    }
}

pub(crate) fn unreadable_record_refusal(count: usize) -> AdapterError {
    AdapterError::new(
        ErrorCode::ActionFailed,
        "A dead input call left a record of held input that cannot be read, so no input was sent",
    )
    .with_suggestion(
        "Check that no key or mouse button is stuck, release it by hand if it is, then retry",
    )
    .with_details(json!({
        "cause": "held_input_record_unreadable",
        "unreadable_records": count,
        "physical_delivery_started": false,
    }))
    .with_disposition(DeliverySemantics::not_delivered())
}

#[cfg(test)]
#[path = "held_journal_tests.rs"]
mod tests;
