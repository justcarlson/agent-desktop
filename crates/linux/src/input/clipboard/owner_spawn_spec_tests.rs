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
