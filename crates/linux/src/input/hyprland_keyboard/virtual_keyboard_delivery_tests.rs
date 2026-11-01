use std::cell::Cell;
use std::rc::Rc;

use agent_desktop_core::{AdapterError, Deadline, DeliverySemantics};

use super::deliver;
use crate::input::delivery::Delivery;
use crate::input::hyprland_keyboard::fake_wire::{FakeWire, WireLog};
use crate::input::keyboard::key_names::{KEYSYM_CONTROL_L, KEYSYM_SHIFT_L};
use crate::input::keyboard::keysym_step::KeysymStep;
use crate::input::wayland::keymap::xkb_keymap::XkbKeymap;

const OWNER_US_RU: &str = include_str!("../wayland/keymap/us_ru.xkb");
const KEYSYM_A: u32 = 0x61;
const KEYSYM_CYRILLIC_YERU: u32 = 0x6d9;
const RUSSIAN: u32 = 1;
const ENGLISH: u32 = 0;

fn live() -> Deadline {
    Deadline::after(5_000).expect("a five second deadline builds")
}

fn edge(keysym: u32, down: bool) -> KeysymStep {
    KeysymStep { keysym, down }
}

fn tap(keysym: u32) -> Vec<KeysymStep> {
    vec![edge(keysym, true), edge(keysym, false)]
}

fn mask(name: &std::ffi::CStr) -> u32 {
    XkbKeymap::compile(OWNER_US_RU.as_bytes())
        .expect("libxkbcommon loads")
        .expect("the keymap compiles")
        .mod_mask(name)
        .expect("the mask reads")
}

fn keys(log: &WireLog) -> Vec<String> {
    log.borrow()
        .iter()
        .filter(|event| event.starts_with("down:") || event.starts_with("up:"))
        .cloned()
        .collect()
}

fn send(
    wire: FakeWire,
    layout: u32,
    chunks: &[&[KeysymStep]],
    gate: &mut dyn FnMut() -> Result<(), AdapterError>,
) -> Result<Delivery, AdapterError> {
    deliver(wire, OWNER_US_RU.to_owned(), layout, chunks, gate, live())
}

#[test]
fn text_types_in_the_active_layout_and_the_keyboard_is_destroyed_after() {
    let wire = FakeWire::new();
    let (log, held) = (Rc::clone(&wire.log), Rc::clone(&wire.held));
    let typed = tap(KEYSYM_CYRILLIC_YERU);
    let chunks: [&[KeysymStep]; 1] = [&typed];
    let delivery = send(wire, RUSSIAN, &chunks, &mut || Ok(())).expect("the text goes");
    assert_eq!(delivery.delivered_events, 2);
    assert!(delivery.virtual_keyboard);
    assert_eq!(keys(&log), ["down:42", "up:42", "down:31", "up:31"]);
    let events = log.borrow().clone();
    assert_eq!(events[..3], ["keymap", "mods:0:1", "sync"]);
    assert!(
        events
            .iter()
            .filter(|e| e.starts_with("mods:"))
            .all(|e| e.ends_with(":1"))
    );
    assert_eq!(events.last().map(String::as_str), Some("destroy"));
    assert!(held.borrow().is_empty());
}

#[test]
fn a_chord_holds_its_modifier_in_the_state_the_keyboard_reports() {
    let control = mask(c"Control");
    let wire = FakeWire::new();
    let log = Rc::clone(&wire.log);
    let chord = [
        edge(KEYSYM_CONTROL_L, true),
        edge(KEYSYM_A, true),
        edge(KEYSYM_A, false),
        edge(KEYSYM_CONTROL_L, false),
    ];
    let chunks: [&[KeysymStep]; 1] = [&chord];
    send(wire, ENGLISH, &chunks, &mut || Ok(())).expect("the chord goes");
    let events = log.borrow().clone();
    let at = |label: &str| events.iter().position(|event| event == label).expect(label);
    let held = format!("mods:{control}:0");
    assert!(at("down:29") < at(&held) && at(&held) < at("down:30"));
    assert!(at("up:30") < at("up:29"));
    assert_eq!(
        keys(&log)[2..],
        ["down:29", "down:30", "up:30", "up:29"].map(str::to_owned)
    );
}

#[test]
fn a_focus_change_mid_batch_stops_releases_every_key_and_is_uncertain() {
    let wire = FakeWire::new();
    let (log, held) = (Rc::clone(&wire.log), Rc::clone(&wire.held));
    let keys_down = [
        edge(KEYSYM_SHIFT_L, true),
        edge(KEYSYM_A, true),
        edge(KEYSYM_A, false),
        edge(KEYSYM_A, true),
        edge(KEYSYM_A, false),
        edge(KEYSYM_SHIFT_L, false),
    ];
    let chunks: [&[KeysymStep]; 1] = [&keys_down];
    let calls = Cell::new(0_u32);
    let mut gate = || {
        calls.set(calls.get() + 1);
        if calls.get() == 6 {
            return Err(AdapterError::internal("focus moved to another window"));
        }
        Ok(())
    };
    let error = send(wire, ENGLISH, &chunks, &mut gate).expect_err("focus moved");
    assert_eq!(error.disposition, DeliverySemantics::uncertain());
    let details = error.details.expect("delivery evidence");
    assert_eq!(details["physical_delivery_started"], true);
    assert_eq!(details["delivered_events"], 5);
    assert!(held.borrow().is_empty());
    assert_eq!(
        keys(&log),
        ["down:42", "up:42", "down:42", "down:30", "up:30", "up:42"]
    );
    let events = log.borrow().clone();
    assert_eq!(events[events.len() - 2..], ["flush", "destroy"]);
    assert_eq!(events.iter().filter(|e| *e == "destroy").count(), 1);
}

#[test]
fn a_keymap_the_compositor_refuses_sends_no_key() {
    let mut wire = FakeWire::new();
    wire.fail_on = Some("keymap");
    let log = Rc::clone(&wire.log);
    let typed = tap(KEYSYM_A);
    let chunks: [&[KeysymStep]; 1] = [&typed];
    let error = send(wire, ENGLISH, &chunks, &mut || Ok(())).expect_err("no keymap");
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
    assert!(keys(&log).is_empty());
}
