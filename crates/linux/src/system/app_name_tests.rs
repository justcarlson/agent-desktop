use super::*;
use crate::system::process_facts::ProcessFacts;

fn facts(comm: &str, exe: Option<&str>) -> ProcessFacts {
    ProcessFacts {
        comm: comm.to_owned(),
        exe_basename: exe.map(str::to_owned),
        instance: "boot:1".to_owned(),
        web_family: None,
    }
}

#[test]
fn the_name_is_the_executable_basename_whatever_the_root_would_say() {
    let process = facts("nautilus", Some("nautilus"));
    assert_eq!(app_name(&process), "nautilus");
}

#[test]
fn the_name_prefers_the_executable_over_the_cut_comm() {
    let process = facts("gnome-terminal-", Some("gnome-terminal-server"));
    assert_eq!(app_name(&process), "gnome-terminal-server");
}

#[test]
fn a_missing_or_blank_executable_falls_back_to_comm() {
    assert_eq!(app_name(&facts("python", None)), "python");
    assert_eq!(app_name(&facts("code", Some("  "))), "code");
}

#[test]
fn a_query_matches_the_name_comm_or_executable_exactly_ignoring_case() {
    let process = facts("gnome-terminal-", Some("gnome-terminal-server"));
    let name = "gnome-terminal-server";
    assert!(name_matches("GNOME-Terminal-Server", name, None, &process));
    assert!(name_matches("gnome-terminal-", name, None, &process));
    let files = facts("nautilus", Some("nautilus"));
    assert!(name_matches("nautilus", "nautilus", None, &files));
}

#[test]
fn the_registry_name_is_a_query_alias_only_while_it_is_known() {
    let files = facts("nautilus", Some("nautilus"));
    assert!(name_matches(
        "org.gnome.nautilus",
        "nautilus",
        Some("org.gnome.Nautilus"),
        &files
    ));
    assert!(!name_matches(
        "org.gnome.nautilus",
        "nautilus",
        None,
        &files
    ));
    assert!(!name_matches("x", "nautilus", Some("  "), &files));
}

#[test]
fn a_substring_does_not_match() {
    let files = facts("nautilus", Some("nautilus"));
    assert!(!name_matches(
        "naut",
        "nautilus",
        Some("org.gnome.Nautilus"),
        &files
    ));
    assert!(!name_matches(
        "Nautilus",
        "x",
        Some("org.gnome.Nautilus"),
        &facts("x", None)
    ));
}
