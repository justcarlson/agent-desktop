use std::cell::Cell;

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
fn an_executable_answers_to_the_name_of_its_entry() {
    let index = AliasIndex::build(&stock());
    assert_eq!(index.names_for("nautilus"), ["Files"]);
    assert_eq!(index.names_for("gnome-calculator"), ["Calculator"]);
    assert_eq!(index.names_for("gnome-text-editor"), ["Text Editor"]);
    assert_eq!(index.names_for("NAUTILUS"), ["Files"]);
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
fn an_env_prefix_is_skipped_and_try_exec_adds_a_second_basename() {
    let index = AliasIndex::build(&[file(
        "t",
        "Name=Tool\nType=Application\nExec=env FOO=1 /opt/app/bin/tool %f\nTryExec=/usr/bin/other",
    )]);
    assert_eq!(index.names_for("tool"), ["Tool"]);
    assert_eq!(index.names_for("other"), ["Tool"]);
    assert!(index.names_for("env").is_empty());
}

#[test]
fn two_entries_with_one_basename_both_match() {
    let index = AliasIndex::build(&[
        file("one", "Name=One\nType=Application\nExec=shared"),
        file("two", "Name=Two\nType=Application\nExec=shared --x"),
    ]);
    assert_eq!(index.names_for("shared"), ["One", "Two"]);
}

#[test]
fn a_name_finds_its_entry_exactly_and_not_by_substring() {
    let index = AliasIndex::build(&stock());
    assert_eq!(index.ids_named("calculator"), ["org.gnome.Calculator"]);
    assert_eq!(index.ids_named("Text Editor"), ["org.gnome.TextEditor"]);
    assert!(index.ids_named("Calc").is_empty());
}

#[test]
fn ten_queries_read_the_disk_once_and_a_change_rebuilds_once() {
    let dir = std::env::temp_dir().join(nonce("alias-cache"));
    std::fs::create_dir_all(&dir).expect("dir");
    let dirs = [dir.clone()];
    let cache = AliasCache::new();
    let reads = Cell::new(0);
    let read = |_: &[PathBuf]| {
        reads.set(reads.get() + 1);
        stock()
    };
    for _ in 0..10 {
        assert_eq!(cache.index(&dirs, read).names_for("nautilus"), ["Files"]);
    }
    assert_eq!((reads.get(), cache.builds()), (1, 1));
    let handle = std::fs::File::open(&dir).expect("open dir");
    handle
        .set_modified(SystemTime::now() + std::time::Duration::from_secs(60))
        .expect("touch");
    for _ in 0..10 {
        cache.index(&dirs, read);
    }
    assert_eq!((reads.get(), cache.builds()), (2, 2));
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
