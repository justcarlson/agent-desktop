use agent_desktop_core::{Deadline, DeliverySemantics};

use super::*;
use crate::input::hyprland_keyboard::fake_wire::FakeWire;

const OWNER_US_RU: &str = include_str!("../wayland/keymap/us_ru.xkb");
const KEY_LEFTSHIFT: u32 = 42;
const KEY_A: u32 = 30;

fn live() -> Deadline {
    Deadline::after(5_000).expect("a five second deadline builds")
}

fn shift_mask() -> u32 {
    XkbKeymap::compile(OWNER_US_RU.as_bytes())
        .expect("libxkbcommon loads")
        .expect("the keymap compiles")
        .mod_mask(c"Shift")
        .expect("the Shift mask reads")
}

#[test]
fn the_keymap_goes_first_then_the_layout_and_a_round_trip_before_any_key() {
    let mut sink = VirtualKeyboardSink::new(FakeWire::new(), 1);
    sink.upload_keymap(OWNER_US_RU, live())
        .expect("the keymap uploads");
    assert_eq!(sink.wire().events(), ["keymap", "mods:0:1", "sync"]);
    assert_eq!(sink.wire().keymaps.borrow().as_slice(), [OWNER_US_RU]);
}

#[test]
fn every_edge_is_flushed_and_a_modifier_change_is_reported_with_the_layout() {
    let shift = shift_mask();
    let mut sink = VirtualKeyboardSink::new(FakeWire::new(), 1);
    sink.upload_keymap(OWNER_US_RU, live())
        .expect("the keymap uploads");
    for (code, down) in [
        (KEY_LEFTSHIFT, true),
        (KEY_A, true),
        (KEY_A, false),
        (KEY_LEFTSHIFT, false),
    ] {
        sink.key(code, down, live()).expect("the key goes");
    }
    assert_eq!(
        sink.wire().events()[3..],
        [
            "down:42".to_owned(),
            format!("mods:{shift}:1"),
            "flush".to_owned(),
            "down:30".to_owned(),
            "flush".to_owned(),
            "up:30".to_owned(),
            "flush".to_owned(),
            "up:42".to_owned(),
            "mods:0:1".to_owned(),
            "flush".to_owned(),
        ]
    );
}

#[test]
fn a_key_before_the_keymap_sends_nothing() {
    let mut sink = VirtualKeyboardSink::new(FakeWire::new(), 0);
    sink.key(KEY_A, true, live()).expect_err("no keymap yet");
    assert!(sink.wire().events().is_empty());
}

#[test]
fn a_keymap_that_does_not_compile_is_refused_before_the_keyboard_exists() {
    let mut sink = VirtualKeyboardSink::new(FakeWire::new(), 0);
    let error = sink
        .upload_keymap("xkb_keymap { broken", live())
        .expect_err("the keymap is refused");
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
    assert!(sink.wire().events().is_empty());
}

#[test]
fn closing_destroys_the_keyboard_once() {
    let mut sink = VirtualKeyboardSink::new(FakeWire::new(), 0);
    sink.upload_keymap(OWNER_US_RU, live())
        .expect("the keymap uploads");
    sink.close(live()).expect("closes");
    sink.close(live()).expect("closes again");
    let destroyed = sink
        .wire()
        .events()
        .iter()
        .filter(|event| *event == "destroy")
        .count();
    assert_eq!(destroyed, 1);
}
