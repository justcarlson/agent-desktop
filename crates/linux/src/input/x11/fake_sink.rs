use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::Duration;

use agent_desktop_core::{AdapterError, Deadline};

use super::pointer_read::PointerRead;

use super::sink_error::SinkError;

use super::sink::X11Sink;
use crate::input::keymap::Keymap;
use crate::input::keymap_edit::KeymapEdit;
use crate::input::plan::Plan;
use crate::input::step::Step;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Recorded {
    Key(u8, bool),
    Button(u8, bool),
    Move(i16, i16),
    Keymap(KeymapEdit),
    RoundTrip,
}

pub(crate) struct FakeWorld {
    pub(crate) events: Vec<Recorded>,
    pub(crate) keymap: Keymap,
    pub(crate) fail_on_event: Option<usize>,
    pub(crate) panic_on_event: Option<usize>,
    pub(crate) root: (u16, u16),
    pub(crate) sticky_buttons: Vec<u8>,
    pub(crate) move_stall: Duration,
    sent: usize,
    held_keys: Vec<u8>,
    held_buttons: Vec<u8>,
    at: (i16, i16),
}

#[derive(Clone)]
pub(crate) struct FakeSink {
    world: Arc<Mutex<FakeWorld>>,
}

pub(crate) fn start_keymap() -> Keymap {
    Keymap::parse(
        8,
        2,
        vec![0x61, 0x41, 0x62, 0x42, 0, 0, 0, 0, 0, 0, 0, 0],
        1,
        vec![50],
    )
}

impl FakeSink {
    pub(crate) fn new() -> Self {
        Self {
            world: Arc::new(Mutex::new(FakeWorld {
                events: Vec::new(),
                keymap: start_keymap(),
                fail_on_event: None,
                panic_on_event: None,
                root: (1280, 800),
                sticky_buttons: Vec::new(),
                move_stall: Duration::ZERO,
                sent: 0,
                held_keys: Vec::new(),
                held_buttons: Vec::new(),
                at: (0, 0),
            })),
        }
    }

    pub(crate) fn world(&self) -> MutexGuard<'_, FakeWorld> {
        self.world
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub(crate) fn recorded(&self) -> Vec<Recorded> {
        self.world().events.clone()
    }

    pub(crate) fn failing_on(self, event: usize) -> Self {
        self.world().fail_on_event = Some(event);
        self
    }

    pub(crate) fn panicking_on(self, event: usize) -> Self {
        self.world().panic_on_event = Some(event);
        self
    }

    fn admit(&self, recorded: Recorded) -> Result<(), SinkError> {
        let mut world = self.world();
        world.sent += 1;
        if world.panic_on_event == Some(world.sent) {
            drop(world);
            panic!("injected panic");
        }
        if world.fail_on_event == Some(world.sent) {
            return Err(SinkError::Request("injected failure".into()));
        }
        world.events.push(recorded);
        Ok(())
    }
}

impl X11Sink for FakeSink {
    fn key(&mut self, code: u8, down: bool) -> Result<(), SinkError> {
        self.admit(Recorded::Key(code, down))?;
        let mut world = self.world();
        world.held_keys.retain(|held| *held != code);
        if down {
            world.held_keys.push(code);
        }
        Ok(())
    }

    fn button(&mut self, code: u8, down: bool) -> Result<(), SinkError> {
        self.admit(Recorded::Button(code, down))?;
        let mut world = self.world();
        world.held_buttons.retain(|held| *held != code);
        if down {
            world.held_buttons.push(code);
        }
        Ok(())
    }

    fn move_to(&mut self, x: i16, y: i16) -> Result<(), SinkError> {
        let stall = self.world().move_stall;
        thread::sleep(stall);
        self.admit(Recorded::Move(x, y))?;
        self.world().at = (x, y);
        Ok(())
    }

    fn round_trip(&mut self) -> Result<(), SinkError> {
        self.world().events.push(Recorded::RoundTrip);
        Ok(())
    }

    fn pointer(&mut self) -> Result<PointerRead, SinkError> {
        let world = self.world();
        let mut mask = 0u16;
        for code in world.held_buttons.iter().chain(world.sticky_buttons.iter()) {
            mask |= 0x80 << code;
        }
        Ok(PointerRead {
            x: i32::from(world.at.0),
            y: i32::from(world.at.1),
            mask,
        })
    }

    fn keys_down(&mut self) -> Result<[u8; 32], SinkError> {
        let world = self.world();
        let mut bits = [0u8; 32];
        for code in &world.held_keys {
            bits[usize::from(*code / 8)] |= 1 << (code % 8);
        }
        Ok(bits)
    }

    fn keymap(&mut self) -> Result<Keymap, SinkError> {
        Ok(self.world().keymap.clone())
    }

    fn edit_keymap(&mut self, edit: &KeymapEdit) -> Result<(), SinkError> {
        self.admit(Recorded::Keymap(edit.clone()))?;
        let mut world = self.world();
        let width = usize::from(world.keymap.keysyms_per_keycode);
        let start = usize::from(edit.keycode - world.keymap.min_keycode) * width;
        world.keymap.keysyms[start..start + width].copy_from_slice(&edit.keysyms);
        Ok(())
    }

    fn root_size(&mut self) -> Result<(u16, u16), SinkError> {
        Ok(self.world().root)
    }
}

pub(crate) fn deadline(ms: u64) -> Deadline {
    Deadline::detached_after(ms).expect("deadline")
}

pub(crate) fn open(
    fake: &FakeSink,
) -> impl FnOnce() -> Result<FakeSink, AdapterError> + Send + 'static {
    let fake = fake.clone();
    move || Ok(fake)
}

pub(crate) fn plan(steps: Vec<Step>, origin: Option<(i32, i32)>) -> Plan {
    Plan {
        steps,
        release_origin: origin,
    }
}

pub(crate) fn drag_plan() -> Plan {
    plan(
        vec![
            Step::MoveTo { x: 10, y: 10 },
            Step::Button {
                code: 1,
                down: true,
            },
            Step::MoveTo { x: 20, y: 20 },
            Step::MoveTo { x: 30, y: 30 },
            Step::Button {
                code: 1,
                down: false,
            },
        ],
        Some((10, 10)),
    )
}

pub(crate) fn events(fake: &FakeSink) -> Vec<Recorded> {
    fake.recorded()
        .into_iter()
        .filter(|event| *event != Recorded::RoundTrip)
        .collect()
}

pub(crate) fn no_gate() -> impl FnMut() -> Result<(), AdapterError> {
    || Ok(())
}
