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
fn an_answering_root_names_the_app() {
    let process = facts("nautilus", Some("nautilus"));
    assert_eq!(
        app_name(Some("org.gnome.Nautilus"), &process),
        "org.gnome.Nautilus"
    );
}

#[test]
fn a_root_that_did_not_answer_falls_back_to_the_executable_not_the_cut_comm() {
    let process = facts("gnome-terminal-", Some("gnome-terminal-server"));
    assert_eq!(app_name(None, &process), "gnome-terminal-server");
}

#[test]
fn a_blank_root_name_falls_back_like_a_missing_one() {
    let process = facts("code", Some("code"));
    assert_eq!(app_name(Some("  "), &process), "code");
    assert_eq!(app_name(Some(""), &facts("python", None)), "python");
}

#[test]
fn a_query_matches_the_name_comm_or_executable_exactly_ignoring_case() {
    let process = facts("gnome-terminal-", Some("gnome-terminal-server"));
    let name = "gnome-terminal-server";
    assert!(name_matches("GNOME-Terminal-Server", name, &process));
    assert!(name_matches("gnome-terminal-", name, &process));
    let files = facts("nautilus", Some("nautilus"));
    assert!(name_matches("nautilus", "org.gnome.Nautilus", &files));
    assert!(name_matches(
        "org.gnome.nautilus",
        "org.gnome.Nautilus",
        &files
    ));
}

#[test]
fn a_substring_does_not_match() {
    let files = facts("nautilus", Some("nautilus"));
    assert!(!name_matches("naut", "org.gnome.Nautilus", &files));
    assert!(!name_matches(
        "Nautilus",
        "org.gnome.Nautilus",
        &facts("x", None)
    ));
}
