use super::{RecordRead, read_record, remove_if_generation};
use crate::system::hyprland::cursor_renderer::registry_fixture::{RegistryFixture, record_for};

#[test]
fn a_missing_record_is_absent_and_a_corrupt_one_unreadable() {
    let fixture = RegistryFixture::new();
    assert_eq!(read_record(&fixture.paths.record), RecordRead::Absent);
    std::fs::write(&fixture.paths.record, b"{").expect("write");
    assert_eq!(read_record(&fixture.paths.record), RecordRead::Unreadable);
}

#[test]
fn removal_leaves_a_newer_generations_files() {
    let fixture = RegistryFixture::new();
    record_for(std::process::id(), None, "newer")
        .write_atomic(&fixture.paths.record)
        .expect("write");
    std::fs::write(&fixture.paths.socket, b"").expect("socket stand-in");
    assert!(!remove_if_generation(&fixture.paths, "older"));
    assert!(fixture.paths.record.exists());
    assert!(fixture.paths.socket.exists());
    assert!(remove_if_generation(&fixture.paths, "newer"));
    assert!(!fixture.paths.record.exists());
    assert!(!fixture.paths.socket.exists());
}
