use std::cell::Cell;
use std::path::PathBuf;

use agent_desktop_core::Deadline;

use super::{GuardOutcome, release_held};
use crate::input::hyprland_keyboard::fake_wire::{FakeWire, WireLog};
use crate::input::hyprland_keyboard::key_guard::guard_record::HeldRecord;

const OWNER_US_RU: &str = include_str!("../../wayland/keymap/us_ru.xkb");
const KEY_LEFTSHIFT: u32 = 42;
const KEY_A: u32 = 30;

fn live() -> Deadline {
    Deadline::after(5_000).expect("a five second deadline builds")
}

fn held(name: &str, codes: &[u32]) -> HeldRecord {
    let dir = std::env::temp_dir().join(format!(
        "ad-key-guard-release-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the folder is created");
    let path = dir.join("77.json");
    std::fs::write(&path, "{}").expect("the record writes");
    HeldRecord {
        path,
        codes: codes.to_vec(),
    }
}

fn wire_with(log: &WireLog, fail_on: Option<&'static str>) -> FakeWire {
    FakeWire {
        log: log.clone(),
        fail_on,
        ..FakeWire::new()
    }
}

#[test]
fn a_held_key_is_released_newest_first_through_a_new_keyboard() {
    let record = held("released", &[KEY_LEFTSHIFT, KEY_A]);
    let path: PathBuf = record.path.clone();
    let log = WireLog::default();

    let outcome = release_held(Some(record), 1, live(), |_| {
        Ok((wire_with(&log, None), OWNER_US_RU.to_owned()))
    })
    .expect("the keys are released");

    assert_eq!(outcome, GuardOutcome::Released(2));
    assert_eq!(
        *log.borrow(),
        [
            "keymap", "mods:0:1", "sync", "up:30", "up:42", "flush", "sync", "destroy"
        ]
    );
    assert!(!path.exists(), "the released record is removed");
}

#[test]
fn a_clean_exit_never_connects() {
    let opened = Cell::new(false);
    let outcome = release_held(None, 0, live(), |_| {
        opened.set(true);
        Ok((FakeWire::new(), OWNER_US_RU.to_owned()))
    })
    .expect("nothing to release");
    assert_eq!(outcome, GuardOutcome::Clean);
    assert!(!opened.get(), "the guard connected with nothing held");
}

#[test]
fn a_release_that_fails_keeps_the_record_and_still_destroys_the_keyboard() {
    let record = held("failed", &[KEY_A]);
    let path = record.path.clone();
    let log = WireLog::default();

    release_held(Some(record), 0, live(), |_| {
        Ok((wire_with(&log, Some("up:30")), OWNER_US_RU.to_owned()))
    })
    .expect_err("the key-up failed");

    assert_eq!(log.borrow().last().map(String::as_str), Some("destroy"));
    assert!(path.exists(), "the next delivery still needs the record");
}

#[test]
fn a_connection_that_fails_keeps_the_record() {
    let record = held("unconnected", &[KEY_A]);
    let path = record.path.clone();
    release_held::<FakeWire, _>(Some(record), 0, live(), |_| {
        Err(agent_desktop_core::AdapterError::internal("no compositor"))
    })
    .expect_err("the guard could not connect");
    assert!(path.exists(), "the next delivery still needs the record");
}
