use super::*;
use crate::system::process_facts::ProcessFacts;
use crate::tree::live_launch::nonce;

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
    assert!(name_matches(
        "GNOME-Terminal-Server",
        name,
        None,
        &process,
        &[]
    ));
    assert!(name_matches("gnome-terminal-", name, None, &process, &[]));
    let files = facts("nautilus", Some("nautilus"));
    assert!(name_matches("nautilus", "nautilus", None, &files, &[]));
}

#[test]
fn launcher_names_find_the_server_exactly_without_desktop_entries() {
    let server = facts("gnome-terminal-", Some("gnome-terminal-server"));
    for query in ["gnome-terminal", "GNOME-TERMINAL", "org.gnome.Terminal"] {
        assert!(aliased(query, &server, &[]), "{query}");
        assert!(!aliased(query, &facts("other", Some("other")), &[]));
        assert!(!aliased(
            query,
            &facts("gnome-terminal-", Some("gnome-terminal-server-helper")),
            &[]
        ));
    }
    assert!(!aliased("gnome-term", &server, &[]));
    assert_eq!(app_name(&server), "gnome-terminal-server");
}

#[test]
fn the_registry_name_is_a_query_alias_only_while_it_is_known() {
    let files = facts("nautilus", Some("nautilus"));
    assert!(name_matches(
        "org.gnome.nautilus",
        "nautilus",
        Some("org.gnome.Nautilus"),
        &files,
        &[]
    ));
    assert!(!name_matches(
        "org.gnome.nautilus",
        "nautilus",
        None,
        &files,
        &[]
    ));
    assert!(!name_matches("x", "nautilus", Some("  "), &files, &[]));
}

#[test]
fn a_substring_does_not_match() {
    let files = facts("nautilus", Some("nautilus"));
    assert!(!name_matches(
        "naut",
        "nautilus",
        Some("org.gnome.Nautilus"),
        &files,
        &[]
    ));
    assert!(!name_matches(
        "Nautilus",
        "x",
        Some("org.gnome.Nautilus"),
        &facts("x", None),
        &[]
    ));
}

#[test]
fn an_executable_deleted_while_it_runs_keeps_its_name() {
    let replaced = facts("calc", Some("gnome-calculator (deleted)"));
    assert_eq!(app_name(&replaced), "gnome-calculator");
    assert!(name_matches(
        "gnome-calculator",
        "gnome-calculator",
        None,
        &replaced,
        &[]
    ));
    assert!(!name_matches(
        "gnome-calculator (deleted)",
        "gnome-calculator",
        None,
        &replaced,
        &[]
    ));
    assert_eq!(app_name(&facts("x", Some(" (deleted)"))), "x");
}

fn aliased(query: &str, process: &ProcessFacts, aliases: &[&str]) -> bool {
    let aliases: Vec<String> = aliases.iter().map(|alias| (*alias).to_owned()).collect();
    name_matches(query, &app_name(process), None, process, &aliases)
}

#[test]
fn an_alias_matches_exactly_ignoring_case_and_never_renames_the_app() {
    let files = facts("nautilus", Some("nautilus"));
    assert!(aliased("Files", &files, &["Files"]));
    assert!(aliased("files", &files, &["Files"]));
    assert!(aliased("nautilus", &files, &["Files"]));
    assert!(!aliased("Files", &files, &[]));
    assert!(!aliased("File", &files, &["Files"]));
    assert!(!aliased("Files Helper", &files, &["Files"]));
    assert_eq!(app_name(&files), "nautilus");
}

#[test]
fn desktop_identity_uses_the_unique_executable_mapping_even_after_an_upgrade() {
    let dir = std::env::temp_dir().join(nonce("app-name-desktop-id"));
    std::fs::create_dir_all(&dir).expect("desktop directory");
    std::fs::write(
        dir.join("org.example.Files.desktop"),
        "[Desktop Entry]\nType=Application\nName=Files\nExec=files %U\n",
    )
    .expect("files entry");
    std::fs::write(
        dir.join("org.example.SharedOne.desktop"),
        "[Desktop Entry]\nType=Application\nName=One\nExec=shared\n",
    )
    .expect("first shared entry");
    std::fs::write(
        dir.join("org.example.SharedTwo.desktop"),
        "[Desktop Entry]\nType=Application\nName=Two\nExec=shared\n",
    )
    .expect("second shared entry");
    let cache = crate::system::alias_cache::AliasCache::new();
    let index = cache.index(std::slice::from_ref(&dir));
    for executable in ["files", "files (deleted)"] {
        assert_eq!(
            desktop_id_in(&index, &facts("files", Some(executable))).as_deref(),
            Some("org.example.Files")
        );
    }
    assert_eq!(
        desktop_id_in(&index, &facts("shared", Some("shared"))),
        None
    );
    assert_eq!(desktop_id_in(&index, &facts("files", None)), None);
    let _ = std::fs::remove_dir_all(dir);
}
