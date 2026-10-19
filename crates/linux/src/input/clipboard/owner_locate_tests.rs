use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};

use agent_desktop_core::{DeliverySemantics, ErrorCode};

use super::*;

fn maps_line(start: usize, end: usize, path: &Path) -> String {
    format!(
        "{start:x}-{end:x} r-xp 00000000 08:01 1234 {}\n",
        path.display()
    )
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ad-locate-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("a scratch directory");
    dir
}

fn install(dir: &Path, mode: u32) -> PathBuf {
    let helper = dir.join(HELPER_NAME);
    fs::write(&helper, "#!/bin/sh\n").expect("a helper file");
    fs::set_permissions(&helper, fs::Permissions::from_mode(mode)).expect("a mode");
    helper
}

fn image_in(dir: &Path, name: &str) -> PathBuf {
    let image = dir.join(name);
    fs::write(&image, "image").expect("an image file");
    image
}

#[test]
fn the_helper_sits_beside_a_library_image_found_in_the_maps() {
    let dir = scratch("library");
    let image = image_in(&dir, "libagent_desktop_ffi.so");
    let helper = install(&dir, 0o755);
    let maps = format!(
        "{}{}",
        maps_line(0x1000, 0x2000, Path::new("/usr/lib/libc.so.6")),
        maps_line(0x7000, 0x9000, &image)
    );
    let found = locate_in(&maps, 0x7abc).expect("the helper is located");
    assert_eq!(found, fs::canonicalize(helper).expect("canonical"));
}

#[test]
fn the_helper_sits_beside_the_cli_binary_the_same_way() {
    let dir = scratch("cli");
    let image = image_in(&dir, "agent-desktop");
    let helper = install(&dir, 0o755);
    let maps = maps_line(0x4000, 0x6000, &image);
    let found = locate_in(&maps, 0x4000).expect("the helper is located");
    assert_eq!(found, fs::canonicalize(helper).expect("canonical"));
}

#[test]
fn an_address_outside_every_mapping_finds_no_image() {
    let maps = maps_line(0x1000, 0x2000, Path::new("/usr/lib/libc.so.6"));
    assert_eq!(image_from_maps(&maps, 0x2000), None);
    assert_eq!(
        image_from_maps(&maps, 0x1fff),
        Some(PathBuf::from("/usr/lib/libc.so.6"))
    );
}

#[test]
fn a_deleted_image_suffix_is_dropped_and_anonymous_mappings_are_skipped() {
    let maps =
        "1000-2000 rw-p 00000000 00:00 0 \n3000-4000 r-xp 0 08:01 9 /opt/x/lib.so (deleted)\n";
    assert_eq!(image_from_maps(maps, 0x1500), None);
    assert_eq!(
        image_from_maps(maps, 0x3500),
        Some(PathBuf::from("/opt/x/lib.so"))
    );
}

fn refused_by(dir: &Path, image: &Path) -> agent_desktop_core::AdapterError {
    let maps = maps_line(0x1000, 0x2000, image);
    let error = locate_in(&maps, 0x1000).expect_err("the helper is refused");
    assert_eq!(error.code, ErrorCode::PlatformNotSupported, "{dir:?}");
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
    assert!(error.message.contains(HELPER_NAME), "{}", error.message);
    error
}

#[test]
fn a_missing_helper_is_refused_by_name() {
    let dir = scratch("missing");
    let image = image_in(&dir, "agent-desktop");
    refused_by(&dir, &image);
}

#[test]
fn a_directory_in_place_of_the_helper_is_refused() {
    let dir = scratch("directory");
    let image = image_in(&dir, "agent-desktop");
    fs::create_dir(dir.join(HELPER_NAME)).expect("a directory");
    refused_by(&dir, &image);
}

#[test]
fn a_symlink_out_of_the_directory_is_refused() {
    let dir = scratch("symlink");
    let elsewhere = scratch("symlink-target");
    let real = install(&elsewhere, 0o755);
    let image = image_in(&dir, "agent-desktop");
    symlink(real, dir.join(HELPER_NAME)).expect("a symlink");
    refused_by(&dir, &image);
}

#[test]
fn a_helper_that_is_not_executable_is_refused() {
    let dir = scratch("plain");
    let image = image_in(&dir, "agent-desktop");
    install(&dir, 0o644);
    let error = refused_by(&dir, &image);
    assert!(
        error.message.contains("not executable"),
        "{}",
        error.message
    );
}

#[test]
fn a_helper_writable_by_others_is_refused_whatever_the_group() {
    for mode in [0o757, 0o777, 0o702, 0o1757] {
        let dir = scratch(&format!("others-{mode:o}"));
        let helper = install(&dir, mode);
        let error = check_helper_with(&helper, &|_| true).expect_err("refused");
        assert!(error.message.contains("by others"), "{}", error.message);
    }
}

#[test]
fn a_group_writable_helper_in_a_shared_group_is_refused() {
    let dir = scratch("shared-group");
    let helper = install(&dir, 0o775);
    let error = check_helper_with(&helper, &|_| false).expect_err("refused");
    assert!(error.message.contains("not private"), "{}", error.message);
    assert_eq!(error.code, ErrorCode::PlatformNotSupported);
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
}

#[test]
fn a_group_writable_helper_in_the_private_group_is_accepted() {
    let dir = scratch("private-group");
    let helper = install(&dir, 0o775);
    check_helper_with(&helper, &|_| true).expect("accepted");
}

#[test]
fn the_group_asked_about_is_the_group_of_the_file() {
    let dir = scratch("asked-group");
    let helper = install(&dir, 0o775);
    let expected = fs::symlink_metadata(&helper).expect("metadata").gid();
    check_helper_with(&helper, &|gid| gid == expected).expect("accepted");
}

#[test]
fn a_helper_without_group_write_never_asks_about_the_group() {
    let dir = directory_with("no-group-write", 0o755);
    let helper = install(&dir, 0o755);
    check_helper_with(&helper, &|_| panic!("the group is not consulted")).expect("accepted");
}

fn directory_with(name: &str, mode: u32) -> PathBuf {
    let dir = scratch(name);
    fs::set_permissions(&dir, fs::Permissions::from_mode(mode)).expect("a directory mode");
    dir
}

#[test]
fn a_directory_writable_by_others_is_refused() {
    for mode in [0o777, 0o757, 0o707] {
        let dir = directory_with(&format!("dir-others-{mode:o}"), mode);
        let helper = install(&dir, 0o755);
        let error = check_helper_with(&helper, &|_| true).expect_err("refused");
        assert!(
            error.message.contains("The directory") && error.message.contains("by others"),
            "{}",
            error.message
        );
    }
}

#[test]
fn a_sticky_directory_writable_by_others_is_accepted() {
    let dir = directory_with("dir-sticky", 0o1777);
    let helper = install(&dir, 0o755);
    check_helper_with(&helper, &|_| true).expect("accepted");
}

#[test]
fn a_sticky_bit_does_not_excuse_a_group_that_is_shared() {
    let dir = directory_with("dir-sticky-group", 0o1770);
    let helper = install(&dir, 0o755);
    let error = check_helper_with(&helper, &|_| false).expect_err("refused");
    assert!(error.message.contains("The directory"), "{}", error.message);
}

#[test]
fn a_group_writable_directory_follows_the_private_group_rule() {
    let dir = directory_with("dir-group", 0o775);
    let helper = install(&dir, 0o755);
    let error = check_helper_with(&helper, &|_| false).expect_err("refused");
    assert!(
        error.message.contains("The directory") && error.message.contains("not private"),
        "{}",
        error.message
    );
    check_helper_with(&helper, &|_| true).expect("accepted");
}

#[test]
fn a_directory_that_is_not_writable_by_others_or_group_never_asks_about_the_group() {
    let dir = directory_with("dir-closed", 0o755);
    let helper = install(&dir, 0o755);
    check_helper_with(&helper, &|_| panic!("the group is not consulted")).expect("accepted");
}

#[test]
fn a_helper_found_through_the_path_or_the_current_image_is_never_used() {
    let dir = scratch("path");
    let image = image_in(&dir, "agent-desktop");
    let elsewhere = scratch("path-bin");
    install(&elsewhere, 0o755);
    let maps = maps_line(0x1000, 0x2000, &image);
    let error = locate_in(&maps, 0x1000).expect_err("a helper elsewhere is not found");
    assert_eq!(error.code, ErrorCode::PlatformNotSupported);
}
