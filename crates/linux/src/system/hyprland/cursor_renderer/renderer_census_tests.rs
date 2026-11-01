use super::census;
use crate::system::hyprland::cursor_renderer::exe_identity::ExeIdentity;
use crate::system::hyprland::cursor_renderer::record_class::RecordClass;
use crate::system::hyprland::cursor_renderer::registry_fixture::{
    RegistryFixture, finish, record_for, sleeper,
};
use crate::system::hyprland::cursor_renderer::renderer_paths::RendererPaths;

#[test]
fn the_census_classifies_every_session_record_and_skips_everything_else() {
    let fixture = RegistryFixture::new();
    let dir = &fixture.paths.dir;
    record_for(std::process::id(), None, "own")
        .write_atomic(&fixture.paths.record)
        .expect("own record");
    let other = RendererPaths::in_dir(dir, "0123456789abcdef").expect("paths");
    let child = sleeper();
    record_for(child.id(), None, "sleeper")
        .write_atomic(&other.record)
        .expect("sleeper record");
    let stale = RendererPaths::in_dir(dir, "fedcba9876543210").expect("paths");
    record_for(std::process::id(), Some(1), "stale")
        .write_atomic(&stale.record)
        .expect("stale record");
    std::fs::write(dir.join("broken.json"), b"{").expect("broken");
    std::fs::write(dir.join("x.start.lock"), b"").expect("lock");

    let mut classes: Vec<(String, &str)> = census(dir, ExeIdentity::current())
        .into_iter()
        .map(|(_, class)| match class {
            RecordClass::Valid(record) => (record.owner.generation, "valid"),
            RecordClass::IncompatibleLive(record) => (record.owner.generation, "incompatible"),
            RecordClass::Stale(record) => (record.owner.generation, "stale"),
        })
        .collect();
    classes.sort_unstable();
    finish(child);
    assert_eq!(
        classes,
        [
            ("own".to_owned(), "valid"),
            ("sleeper".to_owned(), "incompatible"),
            ("stale".to_owned(), "stale"),
        ]
    );
}
