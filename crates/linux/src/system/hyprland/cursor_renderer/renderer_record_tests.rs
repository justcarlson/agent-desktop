use std::os::unix::fs::PermissionsExt;

use super::RendererRecord;
use crate::system::hyprland::cursor_renderer::registry_fixture::{RegistryFixture, record_for};

#[test]
fn a_record_round_trips_through_its_flat_json() {
    let record = record_for(
        std::process::id(),
        Some(42),
        "00112233445566778899aabbccddeeff",
    );
    let json = record.to_json();
    for key in [
        "pid",
        "start_time",
        "exe",
        "generation",
        "protocol",
        "session_id",
        "hypr_signature",
        "wayland_socket",
        "created_at",
    ] {
        assert!(json.get(key).is_some(), "missing {key}");
    }
    let parsed = RendererRecord::parse(json.to_string().as_bytes()).expect("parse");
    assert_eq!(parsed, record);
}

#[test]
fn a_partial_record_does_not_parse() {
    assert!(RendererRecord::parse(br#"{"pid":1,"start_time":2}"#).is_none());
    assert!(RendererRecord::parse(b"not json").is_none());
}

#[test]
fn a_written_record_is_private_and_leaves_no_staging_file() {
    let fixture = RegistryFixture::new();
    let record = record_for(std::process::id(), None, "aa");
    record.write_atomic(&fixture.paths.record).expect("write");
    let mode = std::fs::metadata(&fixture.paths.record)
        .expect("metadata")
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600);
    let names: Vec<_> = std::fs::read_dir(&fixture.paths.dir)
        .expect("dir")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    assert!(
        names.iter().all(|name| !name.ends_with(".tmp")),
        "{names:?}"
    );
}
