use std::path::{Path, PathBuf};

use agent_desktop_core::Deadline;

use super::{HeldKeyJournal, instance_key, journal_dir_for};
use crate::input::hyprland_keyboard::record_claim::claim_record;
use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;

fn binding_in(runtime: &str) -> HyprlandBinding {
    HyprlandBinding {
        signature: "sig".into(),
        ipc_socket: PathBuf::from(format!("{runtime}/hypr/sig/.socket.sock")),
        wayland_socket: PathBuf::from(format!("{runtime}/wayland-1")),
        pid: 42,
        start_time: 7,
    }
}

#[test]
fn records_sit_in_the_runtime_directory_the_binding_found() {
    let dir = journal_dir_for(&binding_in("/run/user/1000")).expect("dir");
    assert_eq!(
        dir,
        Path::new("/run/user/1000/agent-desktop-input/hyprland")
    );
}

#[test]
fn records_follow_the_binding_not_the_environment() {
    let dir = journal_dir_for(&binding_in("/srv/rt")).expect("dir");
    assert_eq!(dir, Path::new("/srv/rt/agent-desktop-input/hyprland"));
}

fn dead(_pid: u32, _instance: &str) -> bool {
    false
}

#[test]
fn recovery_refuses_rather_than_press_while_the_key_guard_holds_the_record() {
    let dir = std::env::temp_dir().join(format!("ad-hypr-journal-guarded-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let writer = HeldKeyJournal::at(dir.clone(), "sig:wayland-1", 111).with_liveness(dead);
    writer.record(&[30]).expect("the record writes");
    let reader = HeldKeyJournal::at(dir.clone(), "sig:wayland-1", 222)
        .with_liveness(dead)
        .with_claim_wait(50);
    let guard = claim_record(
        &dir.join("111.json"),
        Deadline::after(50).expect("a deadline"),
    )
    .expect("no error")
    .expect("the guard claims the record");

    reader
        .stale()
        .expect_err("recovery must not press while the guard releases");
    guard.remove().expect("the guard removes the record");

    let stale = reader.stale().expect("the record is gone");
    assert!(
        stale.codes.is_empty(),
        "the guard already released the keys"
    );
    reader.discard(stale).expect("nothing to remove");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_record_names_the_instance_and_its_wayland_socket() {
    assert_eq!(
        instance_key(&binding_in("/run/user/1000")),
        "sig:/run/user/1000/wayland-1"
    );
}
