use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

use agent_desktop_core::{AdapterError, DeliverySemantics, ErrorCode};
use serde_json::{Value, json};

use super::sink::X11Sink;
use crate::input::held_input::HeldInput;
use crate::input::keymap_edit::KeymapEdit;

const JOURNAL_DIR: &str = "agent-desktop-input";
const RECORD_EXTENSION: &str = "json";
const QUARANTINE_EXTENSION: &str = "unreadable";

#[derive(Debug, Clone)]
pub(crate) struct HeldJournal {
    dir: PathBuf,
    display: String,
    pid: u32,
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
    Own,
    Entry(StaleEntry),
    Unreadable(PathBuf),
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

fn process_is_alive(pid: u32) -> bool {
    Path::new(&format!("/proc/{pid}")).exists()
}

impl HeldJournal {
    pub(crate) fn in_runtime_dir(display: &str) -> Option<Self> {
        let runtime = std::env::var_os("XDG_RUNTIME_DIR").filter(|dir| !dir.is_empty())?;
        Some(Self::at(
            PathBuf::from(runtime).join(JOURNAL_DIR),
            display,
            std::process::id(),
        ))
    }

    pub(crate) fn at(dir: PathBuf, display: &str, pid: u32) -> Self {
        Self {
            dir,
            display: display.to_owned(),
            pid,
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
        let entry = json!({
            "display": self.display,
            "pid": self.pid,
            "keys": held.keys,
            "buttons": held.buttons,
            "remaps": remaps
                .iter()
                .map(|edit| json!({ "keycode": edit.keycode, "keysyms": edit.keysyms }))
                .collect::<Vec<_>>(),
        });
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

    fn classify(&self, path: PathBuf, alive: &dyn Fn(u32) -> bool) -> Option<Record> {
        if path.extension()?.to_str()? != RECORD_EXTENSION {
            return None;
        }
        let pid: u32 = path.file_stem()?.to_str()?.parse().ok()?;
        if pid == self.pid {
            return Some(Record::Own);
        }
        if alive(pid) {
            return None;
        }
        let parsed: Option<Value> = std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok());
        let Some(value) = parsed.filter(|value| value["display"].is_string()) else {
            return Some(Record::Unreadable(path));
        };
        (value["display"].as_str() == Some(self.display.as_str())).then(|| {
            Record::Entry(StaleEntry {
                path,
                keys: codes(&value, "keys"),
                buttons: codes(&value, "buttons"),
                remaps: remaps(&value),
            })
        })
    }

    fn scan(&self, alive: &dyn Fn(u32) -> bool) -> Vec<Record> {
        let Ok(listing) = std::fs::read_dir(&self.dir) else {
            return Vec::new();
        };
        listing
            .flatten()
            .filter_map(|file| self.classify(file.path(), alive))
            .filter(|record| !matches!(record, Record::Own))
            .collect()
    }

    #[cfg(test)]
    pub(crate) fn stale_entries(&self, alive: &dyn Fn(u32) -> bool) -> Vec<StaleEntry> {
        self.scan(alive)
            .into_iter()
            .filter_map(|record| match record {
                Record::Entry(entry) => Some(entry),
                _ => None,
            })
            .collect()
    }

    pub(crate) fn release_stale(&self, sink: &mut dyn X11Sink) -> Result<(), AdapterError> {
        self.release_stale_where(sink, &process_is_alive)
    }

    pub(crate) fn release_stale_where(
        &self,
        sink: &mut dyn X11Sink,
        alive: &dyn Fn(u32) -> bool,
    ) -> Result<(), AdapterError> {
        let mut unreadable = 0_usize;
        for record in self.scan(alive) {
            match record {
                Record::Entry(entry) => release_entry(sink, &entry),
                Record::Unreadable(path) => {
                    quarantine(&path);
                    unreadable += 1;
                }
                Record::Own => {}
            }
        }
        if unreadable == 0 {
            return Ok(());
        }
        Err(unreadable_record_refusal(unreadable))
    }
}

fn release_entry(sink: &mut dyn X11Sink, entry: &StaleEntry) {
    let buttons_released = entry
        .buttons
        .iter()
        .all(|code| sink.button(*code, false).is_ok());
    let keys_released = entry.keys.iter().all(|code| sink.key(*code, false).is_ok());
    let keymap_restored = entry
        .remaps
        .iter()
        .all(|edit| sink.edit_keymap(edit).is_ok());
    if buttons_released && keys_released && keymap_restored && sink.round_trip().is_ok() {
        let _ = std::fs::remove_file(&entry.path);
    }
}

fn quarantine(path: &Path) {
    if std::fs::rename(path, path.with_extension(QUARANTINE_EXTENSION)).is_err() {
        let _ = std::fs::remove_file(path);
    }
}

fn unreadable_record_refusal(count: usize) -> AdapterError {
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
