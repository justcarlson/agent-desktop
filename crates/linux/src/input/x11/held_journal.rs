use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::sink::X11Sink;

const JOURNAL_DIR: &str = "agent-desktop-input";

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

    pub(crate) fn record(&self, keys: &[u8], buttons: &[u8]) {
        if keys.is_empty() && buttons.is_empty() {
            self.clear();
            return;
        }
        let entry = json!({
            "display": self.display,
            "pid": self.pid,
            "keys": keys,
            "buttons": buttons,
        });
        let _ = std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&self.dir);
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(self.own_path());
        if let Ok(mut file) = file {
            let _ = file.write_all(entry.to_string().as_bytes());
        }
    }

    pub(crate) fn clear(&self) {
        let _ = std::fs::remove_file(self.own_path());
    }

    pub(crate) fn stale_entries(&self, alive: &dyn Fn(u32) -> bool) -> Vec<StaleEntry> {
        let Ok(listing) = std::fs::read_dir(&self.dir) else {
            return Vec::new();
        };
        listing
            .flatten()
            .filter_map(|file| {
                let value: Value =
                    serde_json::from_slice(&std::fs::read(file.path()).ok()?).ok()?;
                let pid = u32::try_from(value["pid"].as_u64()?).ok()?;
                let same_display = value["display"].as_str() == Some(self.display.as_str());
                (same_display && pid != self.pid && !alive(pid)).then(|| StaleEntry {
                    path: file.path(),
                    keys: codes(&value, "keys"),
                    buttons: codes(&value, "buttons"),
                })
            })
            .collect()
    }

    pub(crate) fn release_stale(&self, sink: &mut dyn X11Sink) {
        self.release_stale_where(sink, &process_is_alive);
    }

    pub(crate) fn release_stale_where(&self, sink: &mut dyn X11Sink, alive: &dyn Fn(u32) -> bool) {
        for entry in self.stale_entries(alive) {
            let buttons_released = entry
                .buttons
                .iter()
                .all(|code| sink.button(*code, false).is_ok());
            let keys_released = entry.keys.iter().all(|code| sink.key(*code, false).is_ok());
            if buttons_released && keys_released && sink.round_trip().is_ok() {
                let _ = std::fs::remove_file(&entry.path);
            }
        }
    }
}

#[cfg(test)]
#[path = "held_journal_tests.rs"]
mod tests;
