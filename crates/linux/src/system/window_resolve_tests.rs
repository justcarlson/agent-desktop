use agent_desktop_core::{ErrorCode, WindowInfo, WindowState};

use super::verify;

fn window(id: &str, pid: u32, instance: Option<&str>, title: &str) -> WindowInfo {
    WindowInfo {
        id: id.to_owned(),
        title: title.to_owned(),
        app: "a".to_owned(),
        pid: pid.into(),
        process_instance: instance.map(str::to_owned),
        bounds: None,
        state: WindowState::default(),
    }
}

#[test]
fn a_live_window_with_the_same_process_resolves_even_when_retitled() {
    let stored = window("w-1.2-a", 5, Some("b:1"), "Old title");
    let live = vec![window("w-1.2-a", 5, Some("b:1"), "New title")];
    assert_eq!(verify(&stored, live).expect("resolves").title, "New title");
}

#[test]
fn a_different_process_instance_is_window_not_found() {
    let stored = window("w-1.2-a", 5, Some("b:1"), "t");
    let live = vec![window("w-1.2-a", 5, Some("b:2"), "t")];
    assert_eq!(
        verify(&stored, live).expect_err("refused").code,
        ErrorCode::WindowNotFound
    );
}

#[test]
fn a_different_pid_is_window_not_found() {
    let stored = window("w-1.2-a", 5, Some("b:1"), "t");
    let live = vec![window("w-1.2-a", 6, Some("b:1"), "t")];
    assert_eq!(
        verify(&stored, live).expect_err("refused").code,
        ErrorCode::WindowNotFound
    );
}

#[test]
fn a_vanished_window_is_window_not_found() {
    let stored = window("w-1.2-a", 5, Some("b:1"), "t");
    let live = vec![window("w-1.3-a", 5, Some("b:1"), "t")];
    assert_eq!(
        verify(&stored, live).expect_err("gone").code,
        ErrorCode::WindowNotFound
    );
}

#[test]
fn a_stored_window_without_a_process_instance_is_refused() {
    let stored = window("w-1.2-a", 5, None, "t");
    let live = vec![window("w-1.2-a", 5, Some("b:1"), "t")];
    assert_eq!(
        verify(&stored, live).expect_err("refused").code,
        ErrorCode::StaleRef
    );
}
