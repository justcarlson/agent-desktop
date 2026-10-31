use agent_desktop_core::{ErrorCode, WindowInfo, WindowState};

use super::{Liveness, resolve_compositor, verify};

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
    assert_eq!(
        verify(&stored, live, Liveness::Alive)
            .expect("resolves")
            .title,
        "New title"
    );
}

#[test]
fn a_different_pid_is_window_not_found() {
    let stored = window("w-1.2-a", 5, Some("b:1"), "t");
    let live = vec![window("w-1.2-a", 6, Some("b:1"), "t")];
    assert_eq!(
        verify(&stored, live, Liveness::Alive)
            .expect_err("refused")
            .code,
        ErrorCode::WindowNotFound
    );
}

#[test]
fn a_stored_window_without_a_process_instance_is_refused() {
    let stored = window("w-1.2-a", 5, None, "t");
    let live = vec![window("w-1.2-a", 5, Some("b:1"), "t")];
    assert_eq!(
        verify(&stored, live, Liveness::Alive)
            .expect_err("refused")
            .code,
        ErrorCode::StaleRef
    );
}

#[test]
fn a_stopped_process_is_unresponsive_with_or_without_a_listed_window() {
    let stored = window("w-1.2-a", 5, Some("b:1"), "t");
    for live in [Vec::new(), vec![stored.clone()]] {
        let error = verify(&stored, live, Liveness::Stopped).expect_err("stopped");
        assert_eq!(error.code, ErrorCode::AppUnresponsive);
        assert!(error.message.contains("'a'"));
    }
}

#[test]
fn an_absent_window_of_a_gone_process_is_window_not_found() {
    let stored = window("w-1.2-a", 5, Some("b:1"), "t");
    let error = verify(&stored, Vec::new(), Liveness::Gone).expect_err("gone");
    assert_eq!(error.code, ErrorCode::WindowNotFound);
    assert!(error.message.contains("no longer open"));
}

#[test]
fn a_reused_process_id_keeps_the_different_process_refusal_while_the_process_answers() {
    let stored = window("w-1.2-a", 5, Some("b:1"), "t");
    for process in [Liveness::Alive, Liveness::Gone] {
        let live = vec![window("w-1.2-a", 5, Some("b:2"), "t")];
        let error = verify(&stored, live, process).expect_err("refused");
        assert_eq!(error.code, ErrorCode::WindowNotFound);
        assert!(error.message.contains("different process"));
    }
}

#[test]
fn a_malformed_compositor_id_is_refused_without_reading_the_compositor() {
    let stored = window("w-hypr-1-2-a-b", 5, None, "foot");
    let error = resolve_compositor(&stored, || -> Result<Vec<WindowInfo>, _> {
        unreachable!("the compositor is never read for a malformed id")
    })
    .expect_err("refused");
    assert_eq!(error.code, ErrorCode::InvalidArgs);
}

#[test]
fn a_compositor_window_resolves_only_while_its_compositor_lists_it() {
    use crate::system::process::{proc_stat::read_stat, process_facts::instance_token};
    let pid = std::process::id();
    let start = read_stat(pid)
        .ok()
        .flatten()
        .map(|stat| stat.start_time)
        .expect("start time");
    let instance = instance_token(start);
    let stored = window("w-hypr-10-20-a1", pid, Some(&instance), "foot");
    let resolved = resolve_compositor(&stored, || {
        Ok(vec![window(
            "w-hypr-10-20-a1",
            pid,
            Some(&instance),
            "foot ~",
        )])
    })
    .expect("listed");
    assert_eq!(resolved.title, "foot ~");
    let restarted = resolve_compositor(&stored, || {
        Ok(vec![window(
            "w-hypr-11-30-a1",
            pid,
            Some(&instance),
            "foot",
        )])
    })
    .expect_err("old compositor id");
    assert_eq!(restarted.code, ErrorCode::WindowNotFound);
}

fn listed(window: WindowInfo) -> crate::system::registry::listed_window::ListedWindow {
    crate::system::registry::listed_window::ListedWindow {
        app: 0,
        frame: crate::system::x11::FrameFacts {
            size_identity: false,
            pid: window.pid.get(),
            title: window.title.clone(),
            extents: None,
            global_position: false,
        },
        info: window,
        active: false,
        modal: false,
        dialog: false,
        sheet: false,
        unshown: false,
    }
}

#[test]
fn an_accessible_window_the_compositor_identity_refuses_does_not_resolve() {
    let stored = window("w-1.2-a", 5, Some("b:1"), "Notes");
    let live = || vec![listed(window("w-1.2-a", 5, Some("b:1"), "Notes"))];
    let refused = super::resolve_accessible(&stored, live(), Liveness::Alive, |evidence| {
        assert_eq!((evidence.pid, evidence.title.as_str()), (5, "Notes"));
        Err(agent_desktop_core::AdapterError::new(
            ErrorCode::AmbiguousTarget,
            "twins",
        ))
    })
    .expect_err("refused");
    assert_eq!(refused.code, ErrorCode::AmbiguousTarget);
    let resolved = super::resolve_accessible(&stored, live(), Liveness::Alive, |_| Ok(()));
    assert_eq!(
        resolved.map(|found| found.id).ok().as_deref(),
        Some("w-1.2-a")
    );
}
