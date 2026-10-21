use std::sync::Arc;
use std::time::{Duration, SystemTime};

use super::*;
use crate::tree::live_launch::nonce;

fn file(id: &str, text: &str) -> (String, String) {
    (id.to_owned(), format!("[Desktop Entry]\n{text}\n"))
}

fn stock() -> Vec<(String, String)> {
    vec![
        file(
            "org.gnome.Nautilus",
            "Name=Files\nType=Application\nExec=nautilus --new-window %U",
        ),
        file(
            "org.gnome.Calculator",
            "Name=Calculator\nType=Application\nExec=gnome-calculator",
        ),
        file(
            "org.gnome.TextEditor",
            "Name=Text Editor\nType=Application\nExec=gnome-text-editor --gapplication-service %U",
        ),
    ]
}

#[test]
fn a_process_is_queried_by_the_desktop_id_even_when_the_root_reports_another_name() {
    use crate::system::app_name::{aliases_in, name_matches};
    use crate::system::process_facts::ProcessFacts;

    let index = AliasIndex::build(&stock());
    let facts = ProcessFacts {
        comm: "gnome-calculato".to_owned(),
        exe_basename: Some("GNOME-CALCULATOR".to_owned()),
        instance: "boot:42".to_owned(),
        web_family: None,
    };
    let aliases = aliases_in(&index, &facts);
    assert!(aliases.iter().any(|alias| alias == "Calculator"));
    assert!(aliases.iter().any(|alias| alias == "org.gnome.Calculator"));
    assert!(name_matches(
        "org.gnome.Calculator",
        "gnome-calculator",
        Some("gnome-calculator"),
        &facts,
        &aliases
    ));
    assert!(!name_matches(
        "org.gnome.Calc",
        "gnome-calculator",
        Some("gnome-calculator"),
        &facts,
        &aliases
    ));
}

#[test]
fn another_executable_and_a_lookalike_get_no_alias() {
    let index = AliasIndex::build(&stock());
    assert!(index.names_for("files-helper").is_empty());
    assert!(index.names_for("nautilu").is_empty());
    assert!(index.names_for("calculator").is_empty());
}

#[test]
fn hidden_link_and_no_exec_entries_give_no_alias() {
    let index = AliasIndex::build(&[
        file("a", "Name=A\nType=Application\nHidden=true\nExec=prog-a"),
        file("b", "Name=B\nType=Link\nExec=prog-b"),
        file("c", "Name=C\nType=Application"),
    ]);
    for exe in ["prog-a", "prog-b", "c"] {
        assert!(index.names_for(exe).is_empty(), "{exe}");
    }
}

#[test]
fn an_env_prefix_is_skipped_and_try_exec_never_identifies_the_app() {
    let index = AliasIndex::build(&[file(
        "t",
        "Name=Tool\nType=Application\nExec=env FOO=1 /opt/app/bin/tool %f\nTryExec=/usr/bin/other",
    )]);
    assert_eq!(index.names_for("tool"), ["Tool"]);
    assert!(index.names_for("other").is_empty());
    assert!(index.names_for("env").is_empty());
}

#[test]
fn two_entries_with_one_basename_receive_no_name_alias() {
    let index = AliasIndex::build(&[
        file("one", "Name=One\nType=Application\nExec=shared"),
        file("two", "Name=Two\nType=Application\nExec=shared --x"),
    ]);
    assert!(index.names_for("shared").is_empty());
}

#[test]
fn a_changed_directory_rebuilds_the_aliases_while_unchanged_queries_reuse_the_index() {
    let dir = std::env::temp_dir().join(nonce("alias-cache"));
    std::fs::create_dir_all(&dir).expect("dir");
    let path = dir.join("org.example.Calculator.desktop");
    std::fs::write(
        &path,
        "[Desktop Entry]\nName=Before\nType=Application\nExec=gnome-calculator\n",
    )
    .expect("entry");
    let dirs = [dir.clone()];
    let cache = AliasCache::new();
    let before = cache.index(&dirs);
    assert_eq!(before.names_for("gnome-calculator"), ["Before"]);
    assert!(Arc::ptr_eq(&before, &cache.index(&dirs)));
    let handle = std::fs::File::open(&dir).expect("open dir");
    handle
        .set_modified(SystemTime::now() + Duration::from_secs(60))
        .expect("touch");
    std::fs::write(
        &path,
        "[Desktop Entry]\nName=After\nType=Application\nExec=gnome-calculator\n",
    )
    .expect("update entry");
    let after = cache.index(&dirs);
    assert_eq!(after.names_for("gnome-calculator"), ["After"]);
    assert!(!Arc::ptr_eq(&before, &after));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn terminal_answers_to_its_name_through_the_server_it_runs_as() {
    let index = AliasIndex::build(&[file(
        "org.gnome.Terminal",
        "Name=Terminal\nType=Application\nExec=gnome-terminal",
    )]);
    assert_eq!(index.names_for("gnome-terminal-server"), ["Terminal"]);
    assert_eq!(index.names_for("gnome-terminal"), ["Terminal"]);
}

#[test]
fn a_desktop_id_never_aliases_the_try_exec_helper() {
    let index = AliasIndex::build(&[file(
        "org.example.Client",
        "Name=Client\nType=Application\nExec=client\nTryExec=helper",
    )]);
    assert_eq!(index.ids_for("client"), ["org.example.Client"]);
    assert!(index.ids_for("helper").is_empty());
}

#[test]
fn shared_executable_basenames_do_not_receive_desktop_id_aliases() {
    let index = AliasIndex::build(&[
        file("one", "Name=One\nType=Application\nExec=/opt/one/shared"),
        file("two", "Name=Two\nType=Application\nExec=/opt/two/shared"),
    ]);
    assert!(index.ids_for("shared").is_empty());
}
