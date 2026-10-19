use std::os::fd::{AsFd, AsRawFd, OwnedFd};
use std::path::Path;

use super::*;

fn none(_: &str) -> Option<String> {
    None
}

fn spec_with(parent: &dyn Fn(&str) -> Option<String>) -> SpawnSpec {
    build_spec(
        &SpawnRequest {
            helper: Path::new("/opt/x/agent-desktop-linux-clipboard-owner"),
            display: ":77",
            nonce: "abcdef0123456789",
            payload_fd: 9,
        },
        parent,
    )
}

#[test]
fn the_program_is_the_located_helper_with_null_stdio() {
    let spec = spec_with(&none);
    assert_eq!(
        spec.program,
        Path::new("/opt/x/agent-desktop-linux-clipboard-owner")
    );
    assert!(spec.null_stdio);
    assert_ne!(spec.program, Path::new("/proc/self/exe"));
}

#[test]
fn the_environment_is_exactly_the_allowlist() {
    let parent = |name: &str| Some(format!("parent-{name}"));
    let spec = spec_with(&parent);
    let names: Vec<&str> = spec.env.keys().map(String::as_str).collect();
    let mut expected = vec![
        "AGENT_DESKTOP_HOME",
        "AGENT_DESKTOP_SELECTION_BUILD",
        "AGENT_DESKTOP_SELECTION_DISPLAY",
        "AGENT_DESKTOP_SELECTION_NONCE",
        "AGENT_DESKTOP_SELECTION_OWNER",
        "AGENT_DESKTOP_SELECTION_PAYLOAD_FD",
        "DISPLAY",
        "HOME",
        "XAUTHORITY",
        "XDG_RUNTIME_DIR",
    ];
    expected.sort_unstable();
    assert_eq!(names, expected);
}

#[test]
fn an_unrelated_parent_variable_is_dropped_and_an_unset_one_stays_unset() {
    let parent = |name: &str| match name {
        "SSH_AUTH_SOCK" | "PATH" | "AGENT_DESKTOP_INTERACTION_LEASE_FD" => Some("x".to_owned()),
        "HOME" => Some("/home/u".to_owned()),
        _ => None,
    };
    let spec = spec_with(&parent);
    assert!(!spec.env.contains_key("SSH_AUTH_SOCK"));
    assert!(!spec.env.contains_key("PATH"));
    assert!(!spec.env.contains_key("AGENT_DESKTOP_INTERACTION_LEASE_FD"));
    assert!(!spec.env.contains_key("XAUTHORITY"));
    assert_eq!(spec.env.get("HOME").map(String::as_str), Some("/home/u"));
}

#[test]
fn the_display_is_the_one_the_setter_resolved_and_not_the_parents() {
    let parent = |name: &str| (name == "DISPLAY").then(|| ":0".to_owned());
    let spec = spec_with(&parent);
    assert_eq!(spec.env.get("DISPLAY").map(String::as_str), Some(":77"));
    assert_eq!(
        spec.env
            .get("AGENT_DESKTOP_SELECTION_DISPLAY")
            .map(String::as_str),
        Some(":77")
    );
}

#[test]
fn the_marker_identity_descriptor_and_nonce_are_passed() {
    let spec = spec_with(&none);
    assert_eq!(
        spec.env.get(MARKER_ENV).map(String::as_str),
        Some(GENERATION.to_string().as_str())
    );
    assert_eq!(spec.env.get(BUILD_IDENTITY_ENV), Some(&build_identity()));
    assert_eq!(spec.env.get(PAYLOAD_FD_ENV).map(String::as_str), Some("9"));
    assert_eq!(
        spec.env.get(NONCE_ENV).map(String::as_str),
        Some("abcdef0123456789")
    );
}

fn inheritable_descriptor() -> OwnedFd {
    let file = std::fs::File::open("/dev/null").expect("a descriptor");
    let descriptor = OwnedFd::from(file);
    fcntl_setfd(&descriptor, FdFlags::empty()).expect("clear close-on-exec");
    descriptor
}

fn close_on_exec(descriptor: &OwnedFd) -> bool {
    fcntl_getfd(descriptor.as_fd())
        .expect("flags")
        .contains(FdFlags::CLOEXEC)
}

#[test]
fn the_lease_descriptor_a_parent_passed_down_becomes_close_on_exec() {
    let descriptor = inheritable_descriptor();
    assert!(!close_on_exec(&descriptor));
    let number = descriptor.as_raw_fd().to_string();
    let parent = |name: &str| (name == INTERACTION_LEASE_FD_ENV).then(|| number.clone());
    assert!(protect_lease_descriptor(&parent));
    assert!(close_on_exec(&descriptor));
}

#[test]
fn a_foreign_host_with_no_lease_variable_changes_nothing() {
    assert!(!protect_lease_descriptor(&none));
}

#[test]
fn a_number_that_is_not_an_open_descriptor_or_is_a_standard_stream_is_left_alone() {
    for value in ["1", "2", "notanumber", "99999", "-4"] {
        let parent = |name: &str| (name == INTERACTION_LEASE_FD_ENV).then(|| value.to_owned());
        assert!(!protect_lease_descriptor(&parent), "{value}");
    }
}
