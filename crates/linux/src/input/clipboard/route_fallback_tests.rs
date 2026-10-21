use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use super::*;

fn tool_dir(name: &str, tools: &[&str]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adfb-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("a directory");
    for tool in tools {
        let path = dir.join(tool);
        fs::write(&path, "#!/bin/sh\n").expect("a tool");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("executable");
    }
    dir
}

fn fallback(dir: &Path, display: Option<&str>) -> Fallback {
    Fallback {
        search_path: Some(dir.to_path_buf().into_os_string()),
        display: display.map(str::to_owned),
    }
}

#[test]
fn present_tools_keep_the_wayland_route() {
    let dir = tool_dir("present", &[COPY, PASTE]);
    let chosen = fallback(&dir, Some(":5")).route(Route::Wayland, &[COPY, PASTE]);
    assert_eq!(chosen, Route::Wayland);
}

#[test]
fn missing_tools_with_a_display_use_the_x11_route() {
    let dir = tool_dir("absent", &[]);
    let chosen = fallback(&dir, Some(":5")).route(Route::Wayland, &[PASTE]);
    assert_eq!(chosen, Route::X11(":5".into()));
}

#[test]
fn one_missing_tool_of_a_write_is_enough_for_the_fallback() {
    let dir = tool_dir("half", &[PASTE]);
    let chosen = fallback(&dir, Some(":5")).route(Route::Wayland, &[COPY, PASTE]);
    assert_eq!(chosen, Route::X11(":5".into()));
}

#[test]
fn a_read_needs_only_the_paste_tool() {
    let dir = tool_dir("paste-only", &[PASTE]);
    let chosen = fallback(&dir, Some(":5")).route(Route::Wayland, &[PASTE]);
    assert_eq!(chosen, Route::Wayland);
}

#[test]
fn an_x11_route_is_never_changed() {
    let dir = tool_dir("x11", &[]);
    let chosen = fallback(&dir, Some(":9")).route(Route::X11(":1".into()), &[PASTE]);
    assert_eq!(chosen, Route::X11(":1".into()));
}
