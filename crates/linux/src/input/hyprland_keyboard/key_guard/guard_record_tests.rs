use std::path::{Path, PathBuf};

use serde_json::json;

use agent_desktop_core::Deadline;

use super::{HeldRecord, claim_held};

const DISPLAY: &str = "sig:/run/user/1000/wayland-1";
const INSTANCE: &str = "boot:123";

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ad-key-guard-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the folder is created");
    dir
}

fn brief() -> Deadline {
    Deadline::after(50).expect("a short deadline builds")
}

fn codes_of(record: Option<HeldRecord>) -> Option<Vec<u32>> {
    record.map(|record| record.codes)
}

fn write(dir: &Path, pid: u32, value: serde_json::Value) {
    std::fs::write(dir.join(format!("{pid}.json")), value.to_string()).expect("the record writes");
}

#[test]
fn the_commands_own_record_gives_its_held_keys() {
    let dir = scratch("own");
    write(
        &dir,
        77,
        json!({"pid": 77, "instance": INSTANCE, "display": DISPLAY, "keys": [42, 30]}),
    );
    let record = claim_held(&dir, 77, INSTANCE, DISPLAY, brief()).expect("the record is claimed");
    assert_eq!(record.path(), dir.join("77.json"));
    assert_eq!(record.codes, [42, 30]);
}

#[test]
fn a_record_from_a_later_process_under_the_same_id_is_not_taken() {
    let dir = scratch("reused");
    write(
        &dir,
        77,
        json!({"pid": 77, "instance": "boot:999", "display": DISPLAY, "keys": [30]}),
    );
    assert_eq!(
        codes_of(claim_held(&dir, 77, INSTANCE, DISPLAY, brief())),
        None
    );
}

#[test]
fn a_record_for_another_display_is_not_taken() {
    let dir = scratch("display");
    write(
        &dir,
        77,
        json!({"pid": 77, "instance": INSTANCE, "display": "other:wayland-2", "keys": [30]}),
    );
    assert_eq!(
        codes_of(claim_held(&dir, 77, INSTANCE, DISPLAY, brief())),
        None
    );
}

#[test]
fn a_clean_exit_leaves_nothing_to_release() {
    let dir = scratch("clean");
    assert_eq!(
        codes_of(claim_held(&dir, 77, INSTANCE, DISPLAY, brief())),
        None
    );
    write(
        &dir,
        77,
        json!({"pid": 77, "instance": INSTANCE, "display": DISPLAY, "keys": []}),
    );
    assert_eq!(
        codes_of(claim_held(&dir, 77, INSTANCE, DISPLAY, brief())),
        None
    );
}

#[test]
fn a_record_already_removed_is_not_an_error() {
    let dir = scratch("removed");
    write(
        &dir,
        77,
        json!({"pid": 77, "instance": INSTANCE, "display": DISPLAY, "keys": [30]}),
    );
    let record = claim_held(&dir, 77, INSTANCE, DISPLAY, brief()).expect("the record is claimed");
    std::fs::remove_file(dir.join("77.json")).expect("the record is removed");
    record
        .remove()
        .expect("a missing record is already removed");
}
