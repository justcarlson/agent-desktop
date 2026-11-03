use std::path::{Path, PathBuf};

use agent_desktop_core::{Deadline, DeliverySemantics};

use super::*;
use crate::input::hyprland_keyboard::fake_wire::FakeWire;
use crate::input::hyprland_keyboard::held_key_journal::HeldKeyJournal;

const OWNER_US_RU: &str = include_str!("../wayland/keymap/us_ru.xkb");
const KEY_LEFTSHIFT: u32 = 42;
const KEY_A: u32 = 30;

const INSTANCE: &str = "instance_a:wayland-1";

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

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ad-hypr-sink-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_file(&dir);
    dir
}

fn dead(_pid: u32, _instance: &str) -> bool {
    false
}

fn journal(dir: &Path, pid: u32) -> HeldKeyJournal {
    HeldKeyJournal::at(dir.to_path_buf(), INSTANCE, pid).with_liveness(dead)
}

fn recorded(dir: &Path) -> Vec<u32> {
    journal(dir, 999).stale().expect("the record reads").codes
}

#[test]
fn a_delivery_killed_after_a_key_down_leaves_that_key_recorded() {
    let dir = scratch("killed");
    let mut sink = VirtualKeyboardSink::new(FakeWire::new(), 0).journaled(Some(journal(&dir, 111)));
    sink.upload_keymap(OWNER_US_RU, live())
        .expect("the keymap uploads");
    sink.key(KEY_LEFTSHIFT, true, live()).expect("shift goes");
    sink.key(KEY_A, true, live()).expect("a goes");
    std::mem::forget(sink);

    assert_eq!(recorded(&dir), [KEY_LEFTSHIFT, KEY_A]);
}

#[test]
fn a_released_key_leaves_no_record() {
    let dir = scratch("released");
    let mut sink = VirtualKeyboardSink::new(FakeWire::new(), 0).journaled(Some(journal(&dir, 111)));
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

    assert!(recorded(&dir).is_empty());
    assert!(!dir.join("111.json").exists());
}

#[test]
fn a_key_down_whose_record_cannot_be_written_is_not_sent() {
    let dir = scratch("unwritable");
    std::fs::write(&dir, "").expect("a file blocks the journal folder");
    let mut sink = VirtualKeyboardSink::new(FakeWire::new(), 0).journaled(Some(journal(&dir, 111)));
    sink.upload_keymap(OWNER_US_RU, live())
        .expect("the keymap uploads");

    let error = sink
        .key(KEY_A, true, live())
        .expect_err("the record is refused");

    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
    assert!(!sink.wire().events().iter().any(|event| event == "down:30"));
    let _ = std::fs::remove_file(&dir);
}

#[test]
fn an_unreadable_record_of_a_dead_delivery_refuses_before_any_key() {
    let dir = scratch("unreadable");
    std::fs::create_dir_all(&dir).expect("dir");
    std::fs::write(dir.join("5.json"), "").expect("write");
    let mut sink = VirtualKeyboardSink::new(FakeWire::new(), 0).journaled(Some(journal(&dir, 111)));

    let error = sink
        .upload_keymap(OWNER_US_RU, live())
        .expect_err("the record cannot be read");

    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
    let details = error.details.expect("details");
    assert_eq!(details["cause"], "held_input_record_unreadable");
    assert!(
        !sink
            .wire()
            .events()
            .iter()
            .any(|e| e.starts_with("down:") || e.starts_with("up:"))
    );
    assert!(dir.join("5.unreadable").exists());
}

#[test]
fn keys_held_on_another_instance_are_left_alone() {
    let dir = scratch("instance");
    HeldKeyJournal::at(dir.clone(), "instance_b:wayland-1", 111)
        .record(&[KEY_A])
        .expect("record");
    let mut sink = VirtualKeyboardSink::new(FakeWire::new(), 0).journaled(Some(journal(&dir, 222)));

    sink.upload_keymap(OWNER_US_RU, live())
        .expect("the keymap uploads");

    assert_eq!(sink.wire().events(), ["keymap", "mods:0:0", "sync"]);
    assert!(dir.join("111.json").exists());
}
