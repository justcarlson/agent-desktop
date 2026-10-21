use std::os::unix::fs::PermissionsExt;

use super::*;
use crate::system::app_alias::{AliasCache, basename_lower};
use crate::tree::live_launch::nonce;

struct Fixture {
    root: PathBuf,
    search: Search,
    index: std::sync::Arc<AliasIndex>,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn executable(dir: &Path, name: &str) -> PathBuf {
    std::fs::create_dir_all(dir).expect("bin dir");
    let file = dir.join(name);
    std::fs::write(&file, "#!/bin/sh\n").expect("exe");
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755)).expect("mode");
    file
}

fn fixture(entries: &[(&str, &str)], programs: &[&str]) -> Fixture {
    let root = std::env::temp_dir().join(nonce("target"));
    let apps = root.join("apps");
    let bin = root.join("bin");
    std::fs::create_dir_all(&apps).expect("apps dir");
    std::fs::create_dir_all(&bin).expect("bin dir");
    for (id, body) in entries {
        std::fs::write(
            apps.join(format!("{id}.desktop")),
            format!("[Desktop Entry]\n{body}\n"),
        )
        .expect("entry");
    }
    for program in programs {
        executable(&bin, program);
    }
    let search = Search {
        data_dirs: vec![apps],
        path_dirs: vec![bin],
    };
    let index = AliasCache::new().index(&search.data_dirs);
    Fixture {
        root,
        search,
        index,
    }
}

const CALCULATOR: (&str, &str) = (
    "org.gnome.Calculator",
    "Name=Calculator\nType=Application\nExec=gnome-calculator",
);

fn keys_in(fixture: &Fixture, identifier: &str) -> Result<Vec<String>, ErrorCode> {
    resolve(identifier, &[], &fixture.search, &fixture.index)
        .map(|plan| plan.keys)
        .map_err(|error| error.code)
}

fn code_of(result: Result<Vec<String>, ErrorCode>) -> ErrorCode {
    result.expect_err("an error")
}

#[test]
fn a_name_reaches_the_entry_the_command_name_reaches() {
    let fixture = fixture(&[CALCULATOR], &["gnome-calculator"]);
    assert_eq!(
        keys_in(&fixture, "Calculator").unwrap()[0],
        "org.gnome.calculator"
    );
    assert_eq!(
        keys_in(&fixture, "calculator").unwrap()[0],
        "org.gnome.calculator"
    );
    assert_eq!(
        keys_in(&fixture, "gnome-calculator").unwrap(),
        ["gnome-calculator"]
    );
}

#[test]
fn a_path_file_beats_an_entry_name_and_a_desktop_id_beats_a_path_file() {
    let fixture = fixture(&[CALCULATOR], &["Calculator", "org.gnome.Calculator"]);
    assert_eq!(keys_in(&fixture, "Calculator").unwrap(), ["calculator"]);
    assert_eq!(
        keys_in(&fixture, "org.gnome.Calculator").unwrap()[0],
        "org.gnome.calculator"
    );
}

#[test]
fn a_part_of_a_name_matches_nothing() {
    let fixture = fixture(&[CALCULATOR], &[]);
    assert_eq!(code_of(keys_in(&fixture, "Calc")), ErrorCode::AppNotFound);
    assert_eq!(code_of(keys_in(&fixture, "gnome")), ErrorCode::AppNotFound);
}

#[test]
fn a_name_two_entries_share_is_ambiguous() {
    let fixture = fixture(
        &[
            ("one", "Name=Files\nType=Application\nExec=a"),
            ("two", "Name=Files\nType=Application\nExec=b"),
        ],
        &[],
    );
    assert_eq!(
        code_of(keys_in(&fixture, "files")),
        ErrorCode::AmbiguousTarget
    );
}

#[test]
fn a_relative_path_and_a_nul_are_invalid() {
    let fixture = fixture(&[], &[]);
    assert_eq!(
        code_of(keys_in(&fixture, "bin/tool")),
        ErrorCode::InvalidArgs
    );
    assert_eq!(code_of(keys_in(&fixture, "./tool")), ErrorCode::InvalidArgs);
    assert_eq!(code_of(keys_in(&fixture, "to\0ol")), ErrorCode::InvalidArgs);
    assert_eq!(code_of(keys_in(&fixture, "")), ErrorCode::InvalidArgs);
}

#[test]
fn an_absolute_file_is_a_path_and_a_missing_one_is_not_found() {
    let fixture = fixture(&[], &["tool"]);
    let tool = fixture.search.path_dirs[0].join("tool");
    let plan = resolve(
        &tool.to_string_lossy(),
        &[],
        &fixture.search,
        &fixture.index,
    )
    .expect("executable path");
    assert_eq!(plan.argv, [tool.to_string_lossy().as_ref()]);
    assert_eq!(plan.keys, ["tool"]);
    assert_eq!(
        code_of(keys_in(&fixture, "/nonexistent/tool")),
        ErrorCode::AppNotFound
    );
}

#[test]
fn a_desktop_file_outside_the_data_directories_is_refused() {
    let fixture = fixture(&[CALCULATOR], &[]);
    let outside = fixture.root.join("evil.desktop");
    std::fs::write(
        &outside,
        "[Desktop Entry]\nName=E\nType=Application\nExec=sh\n",
    )
    .expect("file");
    let error = resolve(
        &outside.to_string_lossy(),
        &[],
        &fixture.search,
        &fixture.index,
    )
    .err()
    .expect("refused");
    assert_eq!(error.code, ErrorCode::InvalidArgs);
    assert_eq!(
        error.message,
        format!(
            "'{}' is a desktop file outside the XDG data directories and is not launched",
            outside.display()
        )
    );
    let inside = fixture.search.data_dirs[0].join("org.gnome.Calculator.desktop");
    assert_eq!(
        resolve(
            &inside.to_string_lossy(),
            &[],
            &fixture.search,
            &fixture.index
        )
        .expect("trusted entry")
        .keys[0],
        "org.gnome.calculator"
    );
}

fn entry_of(body: &str) -> DesktopEntry {
    parse("x", &format!("[Desktop Entry]\n{body}\n")).expect("entry")
}

#[test]
fn a_terminal_entry_or_one_without_exec_is_unsupported_naming_the_entry() {
    for body in [
        "Name=A\nType=Application\nTerminal=true\nExec=vim",
        "Name=A\nType=Application",
    ] {
        let error = check_entry(&entry_of(body), &[]).expect_err("unsupported");
        assert_eq!(error.code, ErrorCode::ActionNotSupported);
        assert!(error.message.contains("'x'"));
    }
}

#[test]
fn a_hidden_entry_a_link_and_a_missing_try_exec_are_not_found() {
    let fixture = fixture(&[], &["present"]);
    let dirs = &fixture.search.path_dirs;
    for body in [
        "Name=A\nType=Application\nHidden=true\nExec=a",
        "Name=A\nType=Link\nExec=a",
        "Name=A\nType=Application\nExec=a\nTryExec=absent",
        "Name=A\nType=Application\nExec=a\nTryExec=/nonexistent/prog",
    ] {
        let error = check_entry(&entry_of(body), dirs).expect_err("not found");
        assert_eq!(error.code, ErrorCode::AppNotFound, "{body}");
    }
    let present = "Name=A\nType=Application\nExec=a\nTryExec=present";
    assert!(check_entry(&entry_of(present), dirs).is_ok());
}

#[test]
fn an_entry_plan_expands_resolves_and_keys_the_entry() {
    let fixture = fixture(
        &[(
            "org.gnome.TextEditor",
            "Name=Text Editor\nType=Application\nExec=gnome-text-editor --gapplication-service %U\nStartupWMClass=Gnome-Text-Editor\nPath=/tmp",
        )],
        &["gnome-text-editor"],
    );
    let arguments = vec!["--flag".to_owned(), "/a.txt".to_owned()];
    let plan = resolve(
        "org.gnome.TextEditor",
        &arguments,
        &fixture.search,
        &fixture.index,
    )
    .expect("plan");
    let program = fixture.search.path_dirs[0].join("gnome-text-editor");
    assert_eq!(
        plan.argv,
        [
            program.to_string_lossy().as_ref(),
            "--gapplication-service",
            "--flag",
            "/a.txt"
        ]
    );
    assert_eq!(plan.cwd, Some(PathBuf::from("/tmp")));
    assert_eq!(
        plan.keys,
        [
            "org.gnome.texteditor",
            "gnome-text-editor",
            "gnome-text-editor"
        ]
    );
}

#[test]
fn a_command_and_a_path_plan_pass_the_arguments_through() {
    let fixture = fixture(&[], &["tool"]);
    let arguments = vec!["-x".to_owned(), "y".to_owned()];
    let command = resolve("tool", &arguments, &fixture.search, &fixture.index).expect("command");
    assert_eq!(command.argv[1..], ["-x", "y"]);
    assert_eq!(command.keys, ["tool"]);
    let path = fixture.search.path_dirs[0].join("tool");
    let by_path = resolve(
        &path.to_string_lossy(),
        &arguments,
        &fixture.search,
        &fixture.index,
    )
    .expect("path");
    assert_eq!(by_path.argv, [path.to_string_lossy().as_ref(), "-x", "y"]);
    assert_eq!(by_path.keys, ["tool"]);
}

#[test]
fn a_program_missing_from_path_is_not_found() {
    let fixture = fixture(&[CALCULATOR], &[]);
    let error = resolve("org.gnome.Calculator", &[], &fixture.search, &fixture.index)
        .err()
        .expect("not found");
    assert_eq!(error.code, ErrorCode::AppNotFound);
}

#[test]
fn launch_and_alias_keys_share_executable_basename_normalization() {
    let fixture = fixture(
        &[(
            "tool",
            "Name=Tool\nType=Application\nExec=/opt/SomeDir/MiXeD\nTryExec=/",
        )],
        &[],
    );
    let actual = executable(&fixture.root.join("SomeDir"), "MiXeD");
    let launched = resolve(
        &actual.to_string_lossy(),
        &[],
        &fixture.search,
        &fixture.index,
    )
    .expect("existing executable path");
    assert_eq!(launched.keys, ["mixed"]);
    assert_eq!(fixture.index.names_for("mixed"), ["Tool"]);
    assert!(fixture.index.names_for("SomeDir").is_empty());
    assert_eq!(basename_lower("/"), None);
    assert_eq!(
        code_of(keys_in(&fixture, "/")),
        ErrorCode::AppNotFound,
        "a directory is not accepted as an executable path"
    );
}
