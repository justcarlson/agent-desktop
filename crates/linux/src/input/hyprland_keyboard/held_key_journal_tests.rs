use std::path::{Path, PathBuf};

use super::{HeldKeyJournal, instance_key, journal_dir_for};
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
fn a_record_the_key_guard_already_removed_is_discarded_without_a_refusal() {
    let dir = std::env::temp_dir().join(format!("ad-hypr-journal-guarded-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let writer = HeldKeyJournal::at(dir.clone(), "sig:wayland-1", 111).with_liveness(dead);
    writer.record(&[30]).expect("the record writes");
    let reader = HeldKeyJournal::at(dir.clone(), "sig:wayland-1", 222).with_liveness(dead);
    let stale = reader.stale().expect("the record reads");
    assert_eq!(stale.codes, [30]);
    std::fs::remove_file(dir.join("111.json")).expect("the guard removes the record");

    reader
        .discard(&stale)
        .expect("a record already removed is not a refusal");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_record_names_the_instance_and_its_wayland_socket() {
    assert_eq!(
        instance_key(&binding_in("/run/user/1000")),
        "sig:/run/user/1000/wayland-1"
    );
}
