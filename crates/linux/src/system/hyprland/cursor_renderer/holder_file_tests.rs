use std::os::unix::fs::PermissionsExt;

use super::{HolderFile, now_ms};
use crate::system::hyprland::cursor_renderer::holder_view::{any_live, live_holders};
use crate::system::hyprland::cursor_renderer::registry_fixture::{RegistryFixture, start_time_of};

fn holder(nonce: &str, expiry: u64) -> HolderFile {
    let pid = std::process::id();
    HolderFile {
        pid,
        start_time: start_time_of(pid),
        nonce: nonce.to_owned(),
        expiry,
    }
}

#[test]
fn a_holder_file_is_private_and_round_trips_its_frozen_fields() {
    let fixture = RegistryFixture::new();
    fixture.paths.ensure_dirs().expect("dirs");
    let written = holder("n-1", now_ms() + 60_000);
    let path = written.write_in(&fixture.paths.holders).expect("write");
    assert_eq!(path, fixture.paths.holders.join("n-1.json"));
    let mode = std::fs::metadata(&path).expect("meta").permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
    let bytes = std::fs::read(&path).expect("read");
    let value: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
    let mut keys: Vec<&str> = value
        .as_object()
        .expect("object")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(keys, ["expiry", "nonce", "pid", "start_time"]);
    assert_eq!(HolderFile::parse(&bytes), Some(written));
}

#[test]
fn only_an_unexpired_holder_of_a_running_process_is_live() {
    let fixture = RegistryFixture::new();
    fixture.paths.ensure_dirs().expect("dirs");
    let holders = &fixture.paths.holders;
    assert!(!any_live(holders));
    holder("expired", now_ms().saturating_sub(1))
        .write_in(holders)
        .expect("write");
    let mut reused = holder("reused", now_ms() + 60_000);
    reused.start_time += 1;
    reused.write_in(holders).expect("write");
    std::fs::write(holders.join("garbage.json"), b"{").expect("garbage");
    assert!(!any_live(holders));
    holder("live", now_ms() + 60_000)
        .write_in(holders)
        .expect("write");
    let live = live_holders(holders);
    assert_eq!(live.len(), 1);
    assert_eq!(live[0].nonce, "live");
}

#[test]
fn a_nonce_that_is_not_a_plain_name_is_refused() {
    let fixture = RegistryFixture::new();
    for nonce in ["", "../x", "a/b", &"n".repeat(65)] {
        assert!(
            HolderFile::path_in(&fixture.paths.holders, nonce).is_err(),
            "{nonce}"
        );
    }
}
