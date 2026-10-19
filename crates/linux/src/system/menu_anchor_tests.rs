use std::cell::Cell;

use super::{Target, answer, owned_by, targets};
use crate::system::liveness::Liveness;
use crate::system::process_facts::ProcessFacts;
use crate::system::registry_app::{RegistryApp, assemble};
use crate::system::toolkit_id::ToolkitId;
use crate::tree::AtspiElement;
use agent_desktop_core::{AdapterError, WindowInfo, WindowState};

fn app(pid: u32) -> RegistryApp {
    assemble(
        AtspiElement::new(":1.9", "/org/a11y/atspi/accessible/root", pid),
        ProcessFacts {
            comm: "x".to_owned(),
            exe_basename: Some("x".to_owned()),
            instance: "boot:1".to_owned(),
            web_family: None,
        },
        Some(Some("App".to_owned())),
        ToolkitId::default(),
    )
}

fn window(id: &str) -> WindowInfo {
    WindowInfo {
        id: id.to_owned(),
        title: String::new(),
        app: "App".to_owned(),
        pid: 7.into(),
        process_instance: None,
        bounds: None,
        state: WindowState {
            is_focused: false,
            accessible: true,
            minimized: Some(false),
            visible: Some(true),
        },
    }
}

fn root() -> AtspiElement {
    AtspiElement::new(":1.9", "/org/a11y/atspi/accessible/root", 7)
}

#[test]
fn an_unrelated_process_is_not_selected_and_a_second_entry_of_the_target_is() {
    let select = owned_by(7);
    assert!(select(&app(7)));
    assert!(!select(&app(8)));
}

#[test]
fn a_process_with_windows_reads_its_windows_and_not_its_roots() {
    let found = targets(vec![root()], vec![window("a"), window("b")]);
    assert_eq!(found.len(), 2);
    assert!(
        found
            .iter()
            .all(|target| matches!(target, Target::Window(_)))
    );
}

#[test]
fn a_process_with_no_window_is_read_from_its_roots() {
    let found = targets(vec![root()], Vec::new());
    assert!(matches!(found.as_slice(), [Target::Root(_)]));
}

#[test]
fn a_stopped_or_gone_process_collects_and_reads_nothing() {
    for state in [Liveness::Stopped, Liveness::Gone] {
        let collected = Cell::new(false);
        let open = answer(
            state,
            || {
                collected.set(true);
                Ok(Vec::new())
            },
            |_| Ok(true),
        );
        assert_eq!(open.ok(), Some(false));
        assert!(!collected.get());
    }
}

#[test]
fn the_first_open_target_ends_the_read() {
    let reads = Cell::new(0);
    let open = answer(
        Liveness::Alive,
        || {
            Ok(targets(
                Vec::new(),
                vec![window("a"), window("b"), window("c")],
            ))
        },
        |_| {
            reads.set(reads.get() + 1);
            Ok(reads.get() == 2)
        },
    );
    assert_eq!(open.ok(), Some(true));
    assert_eq!(reads.get(), 2);
}

#[test]
fn a_menu_frame_that_is_no_window_still_answers_open() {
    let open = answer(
        Liveness::Alive,
        || Ok(targets(vec![root()], Vec::new())),
        |target| Ok(matches!(target, Target::Root(_))),
    );
    assert_eq!(open.ok(), Some(true));
}

#[test]
fn a_read_failure_is_not_an_answer() {
    let open = answer(
        Liveness::Alive,
        || Ok(targets(Vec::new(), vec![window("a")])),
        |_| Err(AdapterError::timeout("slow")),
    );
    assert!(open.is_err());
}
