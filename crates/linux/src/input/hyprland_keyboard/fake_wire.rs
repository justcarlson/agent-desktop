use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;

use agent_desktop_core::AdapterError;

use super::keyboard_wire::KeyboardWire;
use crate::input::wayland::keymap::modifier_state::ModifierState;

pub(crate) type WireLog = Rc<RefCell<Vec<String>>>;

/// A wire that records every request and the keys the compositor would
/// still hold, and fails the request whose label is `fail_on`.
pub(crate) struct FakeWire {
    pub(crate) log: WireLog,
    pub(crate) held: Rc<RefCell<BTreeSet<u32>>>,
    pub(crate) keymaps: Rc<RefCell<Vec<String>>>,
    pub(crate) fail_on: Option<&'static str>,
}

impl FakeWire {
    pub(crate) fn new() -> Self {
        Self {
            log: WireLog::default(),
            held: Rc::default(),
            keymaps: Rc::default(),
            fail_on: None,
        }
    }

    pub(crate) fn events(&self) -> Vec<String> {
        self.log.borrow().clone()
    }

    fn record(&mut self, label: String) -> Result<(), AdapterError> {
        self.log.borrow_mut().push(label.clone());
        if self.fail_on == Some(label.as_str()) {
            return Err(AdapterError::internal("the fake wire failed this request"));
        }
        Ok(())
    }
}

impl KeyboardWire for FakeWire {
    fn keymap(&mut self, text: &str) -> Result<(), AdapterError> {
        self.keymaps.borrow_mut().push(text.to_owned());
        self.record("keymap".to_owned())
    }

    fn key(&mut self, _time_ms: u32, code: u32, down: bool) -> Result<(), AdapterError> {
        if down {
            self.held.borrow_mut().insert(code);
        } else {
            self.held.borrow_mut().remove(&code);
        }
        let edge = if down { "down" } else { "up" };
        self.record(format!("{edge}:{code}"))
    }

    fn modifiers(&mut self, state: ModifierState) -> Result<(), AdapterError> {
        self.record(format!("mods:{}:{}", state.depressed, state.group))
    }

    fn sync(&mut self) -> Result<(), AdapterError> {
        self.record("sync".to_owned())
    }

    fn flush(&mut self) -> Result<(), AdapterError> {
        self.record("flush".to_owned())
    }

    fn destroy(&mut self) -> Result<(), AdapterError> {
        self.record("destroy".to_owned())
    }
}
