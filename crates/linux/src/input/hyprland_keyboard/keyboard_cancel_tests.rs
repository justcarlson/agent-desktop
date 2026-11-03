use std::cell::Cell;
use std::rc::Rc;

use agent_desktop_core::{Deadline, DeliverySemantics};

use super::stop_on_signal;
use crate::input::hyprland_keyboard::fake_wire::FakeWire;
use crate::input::hyprland_keyboard::virtual_keyboard_delivery::deliver;
use crate::input::hyprland_keyboard::virtual_keyboard_sink::VirtualKeyboardSink;
use crate::input::keyboard::key_names::KEYSYM_SHIFT_L;
use crate::input::keyboard::keysym_step::KeysymStep;
use crate::input::x11::progress::Progress;

const OWNER_US_RU: &str = include_str!("../wayland/keymap/us_ru.xkb");
const KEYSYM_B: u32 = 0x42;
const ENGLISH: u32 = 0;
const AFTER_SHIFT_DOWN_GATE: u32 = 5;

fn live() -> Deadline {
    Deadline::after(5_000).expect("a five second deadline builds")
}

fn edge(keysym: u32, down: bool) -> KeysymStep {
    KeysymStep { keysym, down }
}

#[test]
fn the_gate_passes_until_a_signal_and_refuses_after_without_asking_the_inner_gate() {
    let progress = Progress::default();
    let asked = Cell::new(0);
    let mut inner = || {
        asked.set(asked.get() + 1);
        Ok(())
    };
    let mut gate = stop_on_signal(&mut inner, &progress);
    assert!(gate().is_ok());
    progress.cancel();
    assert!(gate().is_err());
    assert_eq!(asked.get(), 1);
}

#[test]
fn a_signal_during_the_inner_gate_refuses_that_key_down() {
    let progress = Progress::default();
    let mut inner = || {
        progress.cancel();
        Ok(())
    };
    let mut gate = stop_on_signal(&mut inner, &progress);
    assert!(gate().is_err());
}

#[test]
fn a_signal_with_shift_held_releases_shift_and_destroys_the_keyboard() {
    let progress = Progress::default();
    let wire = FakeWire::new();
    let (log, held) = (Rc::clone(&wire.log), Rc::clone(&wire.held));
    let calls = Cell::new(0);
    let mut inner = || {
        calls.set(calls.get() + 1);
        if calls.get() == AFTER_SHIFT_DOWN_GATE {
            progress.cancel();
        }
        Ok(())
    };
    let mut gate = stop_on_signal(&mut inner, &progress);
    let shifted = [
        edge(KEYSYM_SHIFT_L, true),
        edge(KEYSYM_B, true),
        edge(KEYSYM_B, false),
        edge(KEYSYM_SHIFT_L, false),
    ];
    let chunks: [&[KeysymStep]; 2] = [&shifted, &shifted];
    let error = deliver(
        VirtualKeyboardSink::new(wire, ENGLISH),
        OWNER_US_RU.to_owned(),
        &chunks,
        &mut gate,
        live(),
    )
    .expect_err("a signal stops the delivery");

    assert_eq!(error.disposition, DeliverySemantics::uncertain());
    assert!(
        held.borrow().is_empty(),
        "keys stayed held: {:?}",
        held.borrow()
    );
    let events = log.borrow().clone();
    let keys: Vec<&String> = events
        .iter()
        .filter(|event| event.starts_with("down:") || event.starts_with("up:"))
        .collect();
    assert_eq!(
        keys.len(),
        4,
        "the lead tap and the held shift's release only: {keys:?}"
    );
    assert_eq!(keys[2], keys[2].replace("up:", "down:").as_str());
    assert_eq!(keys[3], keys[2].replace("down:", "up:").as_str());
    assert_eq!(events.last().map(String::as_str), Some("destroy"));
}
