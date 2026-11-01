use std::os::unix::fs::symlink;

use super::{KEEP_DRAWN_FLAG, flag_honored, keep_drawn_for_test};
use crate::system::hyprland::cursor_renderer::registry_fixture::RegistryFixture;

#[test]
fn no_flag_keeps_nothing_drawn() {
    let fixture = RegistryFixture::new();
    assert!(!keep_drawn_for_test(&fixture.paths.dir));
}

#[test]
fn an_owned_regular_flag_file_keeps_the_cursor_drawn() {
    let fixture = RegistryFixture::new();
    std::fs::write(fixture.paths.dir.join(KEEP_DRAWN_FLAG), b"").expect("flag");
    assert!(keep_drawn_for_test(&fixture.paths.dir));
}

#[test]
fn a_symlinked_flag_is_ignored() {
    let fixture = RegistryFixture::new();
    let target = fixture.paths.dir.join("elsewhere");
    std::fs::write(&target, b"").expect("target");
    symlink(&target, fixture.paths.dir.join(KEEP_DRAWN_FLAG)).expect("symlink");
    assert!(!keep_drawn_for_test(&fixture.paths.dir));
}

#[test]
fn a_flag_directory_is_ignored() {
    let fixture = RegistryFixture::new();
    std::fs::create_dir(fixture.paths.dir.join(KEEP_DRAWN_FLAG)).expect("dir");
    assert!(!keep_drawn_for_test(&fixture.paths.dir));
}

#[test]
fn a_flag_another_user_owns_is_ignored() {
    let fixture = RegistryFixture::new();
    let flag = fixture.paths.dir.join(KEEP_DRAWN_FLAG);
    std::fs::write(&flag, b"").expect("flag");
    let metadata = std::fs::symlink_metadata(&flag).expect("metadata");
    let user = rustix::process::getuid().as_raw();
    assert!(flag_honored(&metadata, user));
    assert!(!flag_honored(&metadata, user.wrapping_add(1)));
}
