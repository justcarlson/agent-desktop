use agent_desktop_core::AdapterError;

use super::keymap::Keymap;

use super::keymap_edit::KeymapEdit;
use super::release_state::ReleaseState;
use super::x11::held_journal::HeldJournal;
use super::x11::sink::X11Sink;
use super::x11::sink_error::SinkError;

pub(crate) struct ReleaseGuard<'a> {
    sink: &'a mut dyn X11Sink,
    state: ReleaseState,
    held_keys: Vec<u8>,
    held_buttons: Vec<u8>,
    touched: Touched,
    origin: Option<(i32, i32)>,
    remaps: Vec<KeymapEdit>,
    journal: Option<HeldJournal>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct Touched {
    pub(crate) keys: Vec<u8>,
    pub(crate) buttons: Vec<u8>,
}

fn remember(list: &mut Vec<u8>, code: u8) {
    if !list.contains(&code) {
        list.push(code);
    }
}

fn original_keysyms(map: &Keymap, keycode: u8) -> Option<Vec<u32>> {
    let width = usize::from(map.keysyms_per_keycode);
    let row = usize::from(keycode.checked_sub(map.min_keycode)?);
    let start = row.checked_mul(width)?;
    map.keysyms
        .get(start..start.checked_add(width)?)
        .map(<[u32]>::to_vec)
}

fn pointer_coordinate(value: i32) -> Result<i16, SinkError> {
    i16::try_from(value).map_err(|_| SinkError::Request("pointer coordinate out of range".into()))
}

impl<'a> ReleaseGuard<'a> {
    pub(crate) fn new(sink: &'a mut dyn X11Sink) -> Self {
        Self {
            sink,
            state: ReleaseState::default(),
            held_keys: Vec::new(),
            held_buttons: Vec::new(),
            touched: Touched::default(),
            origin: None,
            remaps: Vec::new(),
            journal: None,
        }
    }

    pub(crate) fn journal_to(&mut self, journal: Option<HeldJournal>) {
        self.journal = journal;
    }

    fn sync_journal(&self) -> Result<(), SinkError> {
        match &self.journal {
            Some(journal) => journal
                .record(&self.held_keys, &self.held_buttons)
                .map_err(|error| SinkError::Journal(error.to_string())),
            None => Ok(()),
        }
    }

    pub(crate) fn set_origin(&mut self, origin: Option<(i32, i32)>) {
        self.origin = origin;
    }

    pub(crate) fn delivered(&self) -> u32 {
        self.state.delivered()
    }

    pub(crate) fn touched(&self) -> &Touched {
        &self.touched
    }

    pub(crate) fn sink(&mut self) -> &mut dyn X11Sink {
        self.sink
    }

    pub(crate) fn key(&mut self, code: u8, down: bool) -> Result<(), SinkError> {
        if down {
            self.state.arm();
            let was_held = self.held_keys.contains(&code);
            remember(&mut self.held_keys, code);
            remember(&mut self.touched.keys, code);
            if let Err(error) = self.sync_journal() {
                if !was_held {
                    self.held_keys.retain(|held| *held != code);
                }
                return Err(error);
            }
        }
        self.sink.key(code, down)?;
        self.state.mark_delivered();
        if !down {
            self.held_keys.retain(|held| *held != code);
            let _ = self.sync_journal();
        }
        Ok(())
    }

    pub(crate) fn button(&mut self, code: u8, down: bool) -> Result<(), SinkError> {
        if down {
            self.state.arm();
            let was_held = self.held_buttons.contains(&code);
            remember(&mut self.held_buttons, code);
            remember(&mut self.touched.buttons, code);
            if let Err(error) = self.sync_journal() {
                if !was_held {
                    self.held_buttons.retain(|held| *held != code);
                }
                return Err(error);
            }
        }
        self.sink.button(code, down)?;
        self.state.mark_delivered();
        if !down {
            self.held_buttons.retain(|held| *held != code);
            let _ = self.sync_journal();
        }
        Ok(())
    }

    pub(crate) fn move_to(&mut self, x: i32, y: i32) -> Result<(), SinkError> {
        let (x, y) = (pointer_coordinate(x)?, pointer_coordinate(y)?);
        self.sink.move_to(x, y)?;
        self.state.mark_delivered();
        Ok(())
    }

    pub(crate) fn edit_keymap(&mut self, edit: &KeymapEdit) -> Result<(), SinkError> {
        let current = self.sink.keymap()?;
        let original = original_keysyms(&current, edit.keycode);
        self.state.arm();
        if let Some(original) = original {
            self.track_remap(edit, original);
        }
        self.sink.edit_keymap(edit)?;
        self.state.mark_delivered();
        Ok(())
    }

    fn track_remap(&mut self, edit: &KeymapEdit, original: Vec<u32>) {
        let pending = self
            .remaps
            .iter()
            .position(|saved| saved.keycode == edit.keycode);
        match pending {
            Some(index) if self.remaps[index].keysyms == edit.keysyms => {
                self.remaps.remove(index);
            }
            Some(_) => {}
            None if original == edit.keysyms => {}
            None => self.remaps.push(KeymapEdit {
                keycode: edit.keycode,
                keysyms: original,
            }),
        }
    }

    pub(crate) fn finish(&mut self) -> Result<(), SinkError> {
        let outcome = self.release_everything();
        if outcome.is_ok() {
            self.state.disarm();
        }
        outcome
    }

    pub(crate) fn abort(&mut self, error: AdapterError, suggestion: &str) -> AdapterError {
        let enriched = self.state.enrich_error(error, suggestion);
        let _ = self.release_everything();
        self.state.disarm();
        enriched
    }

    fn release_everything(&mut self) -> Result<(), SinkError> {
        let mut outcome = Ok(());
        if !self.held_buttons.is_empty() {
            if let Some((x, y)) = self.origin {
                keep_first(&mut outcome, self.move_to(x, y));
            }
            while let Some(code) = self.held_buttons.pop() {
                keep_first(&mut outcome, self.sink.button(code, false));
            }
        }
        while let Some(code) = self.held_keys.pop() {
            keep_first(&mut outcome, self.sink.key(code, false));
        }
        while let Some(edit) = self.remaps.pop() {
            keep_first(&mut outcome, self.sink.edit_keymap(&edit));
        }
        keep_first(&mut outcome, self.sink.round_trip());
        if outcome.is_ok() {
            let _ = self.sync_journal();
        }
        outcome
    }
}

fn keep_first(outcome: &mut Result<(), SinkError>, next: Result<(), SinkError>) {
    if outcome.is_ok() {
        *outcome = next;
    }
}

impl Drop for ReleaseGuard<'_> {
    fn drop(&mut self) {
        if self.state.should_release() {
            let _ = self.release_everything();
            self.state.disarm();
        }
    }
}

#[cfg(test)]
#[path = "release_guard_tests.rs"]
mod tests;
