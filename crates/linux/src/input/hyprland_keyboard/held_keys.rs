//! The keys one virtual keyboard holds, kept in its journal so a killed
//! delivery's keys can be released by the next one.

use agent_desktop_core::AdapterError;

use super::held_key_journal::HeldKeyJournal;
use super::keyboard_wire::KeyboardWire;
use super::stale_key_release::release_stale_keys;
use crate::input::x11::held_journal::recovery_refusal;

/// The held keys and, when the session has a runtime folder, their journal.
#[derive(Debug, Default)]
pub(crate) struct HeldKeys {
    journal: Option<HeldKeyJournal>,
    codes: Vec<u32>,
}

impl HeldKeys {
    pub(crate) fn new(journal: Option<HeldKeyJournal>) -> Self {
        Self {
            journal,
            codes: Vec::new(),
        }
    }

    /// Records `code` as held before its key-down is sent.
    pub(crate) fn press(&mut self, code: u32) -> Result<(), AdapterError> {
        if self.codes.contains(&code) {
            return Ok(());
        }
        self.codes.push(code);
        let Some(journal) = &self.journal else {
            return Ok(());
        };
        journal.record(&self.codes).inspect_err(|_| {
            self.codes.pop();
        })
    }

    /// Drops `code` from the record after its key-up is sent.
    pub(crate) fn release(&mut self, code: u32) {
        let before = self.codes.len();
        self.codes.retain(|held| *held != code);
        if let Some(journal) = &self.journal
            && self.codes.len() != before
        {
            let _ = journal.record(&self.codes);
        }
    }

    /// Releases over `wire` the keys that dead deliveries on this instance
    /// left held, then removes their records.
    pub(crate) fn release_stale<W: KeyboardWire>(
        &self,
        wire: &mut W,
        time_ms: u32,
    ) -> Result<(), AdapterError> {
        let Some(journal) = &self.journal else {
            return Ok(());
        };
        let stale = journal.stale()?;
        if !stale.codes.is_empty() {
            release_stale_keys(wire, time_ms, &stale.codes).map_err(recovery_refusal)?;
        }
        journal.discard(&stale)
    }
}
