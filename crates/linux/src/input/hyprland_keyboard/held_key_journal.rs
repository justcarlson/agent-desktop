//! The record of keys a Hyprland virtual keyboard holds. Hyprland keeps a
//! destroyed virtual keyboard's pressed keys held unless
//! `input:virtualkeyboard:release_pressed_on_close` is set, and a killed
//! delivery runs no cleanup. Each key-down is recorded before it is sent, so
//! the next delivery on the same instance can release what a dead one left.
//! The records sit beside the X11 ones and share their file handling.

use std::path::PathBuf;

use agent_desktop_core::{AdapterError, DeliverySemantics};
use serde_json::{Map, Value, json};

use crate::input::x11::held_journal::{
    HeldJournal, JournalFile, journal_dir_in, process_is_alive, quarantine, recovery_refusal,
    unreadable_record_refusal,
};
use crate::input::x11::sink_error::SinkError;
use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;

const HYPRLAND_DIR: &str = "hyprland";

/// The records folder in the runtime directory the binding found, which holds
/// `hypr/<signature>/.socket.sock`. A caller started by SSH or a systemd unit
/// has no `XDG_RUNTIME_DIR` but still records where the session does.
pub(crate) fn journal_dir_for(binding: &HyprlandBinding) -> Option<PathBuf> {
    let runtime = binding.ipc_socket.ancestors().nth(3)?;
    Some(journal_dir_in(runtime).join(HYPRLAND_DIR))
}

/// The records of one Hyprland instance and Wayland display, written as this
/// process.
#[derive(Debug, Clone)]
pub(crate) struct HeldKeyJournal {
    files: HeldJournal,
    alive: fn(u32, &str) -> bool,
}

/// The keys every recoverable record holds, and the records to remove once
/// they are released.
#[derive(Debug, Default)]
pub(crate) struct StaleKeys {
    paths: Vec<PathBuf>,
    pub(crate) codes: Vec<u32>,
}

/// The display a record names: the Hyprland instance and its Wayland socket.
pub(crate) fn instance_key(binding: &HyprlandBinding) -> String {
    format!("{}:{}", binding.signature, binding.wayland_socket.display())
}

pub(crate) fn key_codes(value: &Value) -> impl Iterator<Item = u32> + '_ {
    value["keys"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|code| code.as_u64().and_then(|code| u32::try_from(code).ok()))
}

fn journal_error(error: std::io::Error) -> AdapterError {
    SinkError::Journal(error.to_string()).into_adapter()
}

impl HeldKeyJournal {
    pub(crate) fn in_runtime_dir(binding: &HyprlandBinding) -> Option<Self> {
        Some(Self::at(
            journal_dir_for(binding)?,
            &instance_key(binding),
            std::process::id(),
        ))
    }

    pub(crate) fn at(dir: PathBuf, instance: &str, pid: u32) -> Self {
        Self {
            files: HeldJournal::at(dir, instance, pid),
            alive: process_is_alive,
        }
    }

    #[cfg(test)]
    pub(crate) fn with_liveness(self, alive: fn(u32, &str) -> bool) -> Self {
        Self { alive, ..self }
    }

    /// Records `codes` as held, or removes the record when none is. A key
    /// whose record fails must not be sent.
    pub(crate) fn record(&self, codes: &[u32]) -> Result<(), AdapterError> {
        if codes.is_empty() {
            self.files.clear();
            return Ok(());
        }
        let mut fields = Map::new();
        fields.insert("keys".to_owned(), json!(codes));
        self.files.write(fields).map_err(|error| {
            journal_error(error).with_disposition(DeliverySemantics::not_delivered())
        })
    }

    /// The keys that dead deliveries on this instance, or this process's
    /// previous delivery, left held. A record that cannot be read is set aside
    /// and refuses the delivery, because its keys are unknown.
    pub(crate) fn stale(&self) -> Result<StaleKeys, AdapterError> {
        let mut stale = StaleKeys::default();
        let mut unreadable = 0_usize;
        for file in self.files.files(&self.alive) {
            match file {
                JournalFile::Own(path, value) | JournalFile::Stale(path, value) => {
                    for code in key_codes(&value) {
                        if !stale.codes.contains(&code) {
                            stale.codes.push(code);
                        }
                    }
                    stale.paths.push(path);
                }
                JournalFile::Unreadable(path) => {
                    quarantine(&path);
                    unreadable += 1;
                }
            }
        }
        if unreadable > 0 {
            return Err(unreadable_record_refusal(unreadable));
        }
        Ok(stale)
    }

    /// Removes the records whose keys were released. A record already gone
    /// was removed by the key guard of the delivery that wrote it.
    pub(crate) fn discard(&self, stale: &StaleKeys) -> Result<(), AdapterError> {
        for path in &stale.paths {
            match std::fs::remove_file(path) {
                Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                    return Err(recovery_refusal(journal_error(error)));
                }
                _ => {}
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "held_key_journal_tests.rs"]
mod tests;
