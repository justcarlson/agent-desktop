use agent_desktop_core::{AdapterError, Deadline, ErrorCode};

use super::extended_keymap::ExtendedKeymap;
use super::keycode_sink::KeycodeSink;
use super::owner_keymap::extend;
use super::press_state::PressState;
use crate::input::keyboard::key_names::KEYSYM_SHIFT_L;
use crate::input::keyboard::keysym_step::KeysymStep;
use crate::input::wayland::key_session::KeySession;

const RELEASE_BUDGET_MS: u64 = 1_000;

/// A keyboard delivery over a [`KeycodeSink`]. `create` extends the seat
/// owner's keymap with the keysyms the delivery needs and refuses before any
/// event when it cannot; `start` uploads the extended keymap; each keysym is
/// sent as the evdev edges the key table plans for it.
pub(crate) struct KeycodeSession<S: KeycodeSink> {
    sink: S,
    owner: String,
    layout: u32,
    keysyms: Vec<u32>,
    extended: Option<ExtendedKeymap>,
    uploaded: bool,
    press: PressState,
}

impl<S: KeycodeSink> KeycodeSession<S> {
    /// A session that will type `chunks` under layout `layout` of the owner
    /// keymap text `owner`.
    pub(crate) fn new(sink: S, owner: String, layout: u32, chunks: &[&[KeysymStep]]) -> Self {
        let mut keysyms = vec![KEYSYM_SHIFT_L];
        for step in chunks.iter().flat_map(|chunk| chunk.iter()) {
            if !keysyms.contains(&step.keysym) {
                keysyms.push(step.keysym);
            }
        }
        Self {
            sink,
            owner,
            layout,
            keysyms,
            extended: None,
            uploaded: false,
            press: PressState::default(),
        }
    }

    #[cfg(test)]
    pub(crate) fn sink(&self) -> &S {
        &self.sink
    }
}

fn unbuilt() -> AdapterError {
    AdapterError::internal("the keyboard session was used before its keymap was built")
}

impl<S: KeycodeSink> KeySession for KeycodeSession<S> {
    fn create(&mut self, _deadline: Deadline) -> Result<(), AdapterError> {
        self.extended = Some(extend(&self.owner, &self.keysyms, self.layout)?);
        Ok(())
    }

    fn start(&mut self, deadline: Deadline) -> Result<(), AdapterError> {
        if self.uploaded {
            return Ok(());
        }
        let extended = self.extended.as_ref().ok_or_else(unbuilt)?;
        self.sink.upload_keymap(&extended.text, deadline)?;
        self.uploaded = true;
        Ok(())
    }

    fn keysym(&mut self, keysym: u32, down: bool, deadline: Deadline) -> Result<(), AdapterError> {
        if !self.uploaded {
            return Err(AdapterError::internal(
                "the keyboard session sent a key before its keymap",
            ));
        }
        let before = self.press.clone();
        let extended = self.extended.as_ref().ok_or_else(unbuilt)?;
        let Some(edges) = extended.table.plan(keysym, down, &mut self.press) else {
            return Err(AdapterError::new(
                ErrorCode::ActionNotSupported,
                format!("The keyboard keymap has no key for keysym {keysym:#x}"),
            ));
        };
        for (index, (code, edge_down)) in edges.iter().copied().enumerate() {
            if let Err(error) = self.sink.key(code, edge_down, deadline) {
                self.press = before;
                if down {
                    self.release(&edges[..=index]);
                }
                return Err(error);
            }
        }
        Ok(())
    }

    fn stop(&mut self, _deadline: Deadline) -> Result<(), AdapterError> {
        Ok(())
    }
}

impl<S: KeycodeSink> KeycodeSession<S> {
    fn release(&mut self, pressed: &[(u32, bool)]) {
        let Ok(deadline) = Deadline::detached_after(RELEASE_BUDGET_MS) else {
            return;
        };
        for (code, _) in pressed.iter().rev() {
            let _ = self.sink.key(*code, false, deadline);
        }
    }
}

#[cfg(test)]
#[path = "keycode_session_tests.rs"]
mod tests;
