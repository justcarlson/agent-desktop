use std::cell::RefCell;

use agent_desktop_core::{AdapterError, ErrorCode};

use super::{chosen_apps, gather, pass_over};
use crate::system::listed_window::ListedWindow;
use crate::system::process_facts::ProcessFacts;
use crate::system::registry::Registry;
use crate::system::registry_app::{RegistryApp, assemble};
use crate::system::registry_frames::{FrameRead, RootRead};
use crate::system::toolkit_id::ToolkitId;
use crate::system::window_listing::collect;
use crate::system::x11::X11State;
use crate::tree::test_support::{read_of, words};
use crate::tree::{AtspiElement, Field, RawNode};

fn app(bus: &str, pid: u32, exe: &str, answered: bool) -> RegistryApp {
    assemble(
        AtspiElement::new(bus, "/org/a11y/atspi/accessible/root", pid),
        ProcessFacts {
            comm: exe.to_owned(),
            exe_basename: Some(exe.to_owned()),
            instance: "boot:1".to_owned(),
            web_family: None,
        },
        answered.then(|| Some(exe.to_owned())),
        ToolkitId::default(),
    )
}

fn registry(apps: Vec<RegistryApp>) -> Registry {
    Registry {
        apps,
        skipped: Vec::new(),
    }
}

fn frame(bus: &str, path: &str, pid: u32, state_bits: &[u32]) -> RawNode {
    let mut node = read_of(23, "Doc", &[]).node;
    node.element = AtspiElement::new(bus, path, pid);
    node.traits.interfaces = Field::Present(vec![crate::tree::COMPONENT_INTERFACE.to_owned()]);
    node.traits.states = Field::Present(words(state_bits));
    node
}

fn read(nodes: Vec<RawNode>, read: RootRead) -> FrameRead {
    FrameRead { nodes, read }
}

fn gathered(
    registry: &Registry,
    select: impl Fn(&RegistryApp) -> bool,
    frames: Vec<FrameRead>,
) -> (Vec<ListedWindow>, Vec<super::AppOutcome>, Vec<String>) {
    let chosen = chosen_apps(registry, select, |_| false);
    let asked = RefCell::new(Vec::new());
    let (windows, outcomes) = gather(&chosen, |roots| {
        asked
            .borrow_mut()
            .extend(roots.iter().map(|root| root.bus_name().to_owned()));
        Ok(frames)
    })
    .expect("gathers");
    (windows, outcomes, asked.into_inner())
}

#[test]
fn the_reader_is_never_asked_for_an_excluded_root() {
    let registry = registry(vec![
        app(":1.1", 1, "alpha", true),
        app(":1.2", 2, "beta", true),
    ]);
    let (_, _, asked) = gathered(
        &registry,
        |app| app.root.pid() == 2,
        vec![read(Vec::new(), RootRead::Read)],
    );
    assert_eq!(asked, [":1.2"]);
}

#[test]
fn an_unanswering_stopped_app_is_not_read_and_a_slow_alive_one_is() {
    let registry = registry(vec![
        app(":1.1", 1, "alpha", false),
        app(":1.2", 2, "beta", false),
        app(":1.3", 3, "gamma", true),
    ]);
    let chosen = chosen_apps(&registry, |_| true, |pid| pid == 1 || pid == 3);
    let pids: Vec<u32> = chosen.iter().map(|(_, app)| app.root.pid()).collect();
    assert_eq!(pids, [2, 3]);
}

#[test]
fn a_helper_app_is_not_chosen() {
    let registry = registry(vec![app(":1.1", 1, "gnome-shell", true)]);
    assert!(chosen_apps(&registry, |_| true, |_| false).is_empty());
}

#[test]
fn each_app_keeps_its_own_outcome_and_a_slow_app_never_hides_another() {
    let registry = registry(vec![
        app(":1.1", 1, "alpha", true),
        app(":1.2", 2, "beta", true),
    ]);
    let frames = vec![
        read(vec![frame(":1.1", "/f/1", 1, &[25])], RootRead::TimedOut),
        read(Vec::new(), RootRead::Read),
    ];
    let (windows, outcomes, _) = gathered(&registry, |_| true, frames);
    assert_eq!(windows.len(), 1);
    let reads: Vec<(usize, RootRead)> = outcomes.iter().map(|o| (o.app, o.read)).collect();
    assert_eq!(reads, [(0, RootRead::TimedOut), (1, RootRead::Read)]);
}

#[test]
fn outcome_indices_follow_the_registry_not_the_chosen_list() {
    let registry = registry(vec![
        app(":1.1", 1, "alpha", true),
        app(":1.2", 2, "beta", true),
    ]);
    let (_, outcomes, _) = gathered(
        &registry,
        |app| app.root.pid() == 2,
        vec![read(Vec::new(), RootRead::TimedOut)],
    );
    assert_eq!(outcomes.len(), 1);
    assert_eq!(outcomes[0].app, 1);
}

#[test]
fn two_frames_of_one_app_give_distinct_ids_and_the_apps_instance() {
    let registry = registry(vec![app(":1.1", 1, "alpha", true)]);
    let frames = vec![read(
        vec![
            frame(":1.1", "/org/a11y/atspi/accessible/1", 1, &[25]),
            frame(":1.1", "/org/a11y/atspi/accessible/2", 1, &[25]),
        ],
        RootRead::Read,
    )];
    let (windows, _, _) = gathered(&registry, |_| true, frames);
    let [first, second] = windows.as_slice() else {
        panic!("two windows")
    };
    assert_ne!(first.info.id, second.info.id);
    assert_eq!(first.info.process_instance.as_deref(), Some("boot:1"));
    assert_eq!(second.info.process_instance.as_deref(), Some("boot:1"));
}

#[test]
fn the_pass_lists_what_collect_lists_for_a_fixed_node_set() {
    let registry = registry(vec![app(":1.1", 1, "alpha", true)]);
    let nodes = || {
        vec![
            frame(":1.1", "/org/a11y/atspi/accessible/1", 1, &[25, 1]),
            frame(":1.1", "/org/a11y/atspi/accessible/2", 1, &[25]),
        ]
    };
    let (windows, _, _) = gathered(&registry, |_| true, vec![read(nodes(), RootRead::Read)]);
    let direct = collect(&[(0, &registry.apps[0])], &[nodes()]);
    let ids = |list: &[ListedWindow]| -> Vec<(String, bool)> {
        list.iter()
            .map(|window| (window.info.id.clone(), window.active))
            .collect()
    };
    assert_eq!(ids(&windows), ids(&direct));
}

#[test]
fn a_failed_read_ends_the_pass() {
    let registry = registry(vec![app(":1.1", 1, "alpha", true)]);
    let chosen = chosen_apps(&registry, |_| true, |_| false);
    let error = gather(&chosen, |_| {
        Err(AdapterError::new(ErrorCode::Timeout, "late"))
    })
    .err()
    .expect("fails");
    assert_eq!(error.code, ErrorCode::Timeout);
}

#[test]
fn one_active_candidate_without_x11_evidence_is_the_focused_window() {
    let registry = registry(vec![app(":1.1", 1, "alpha", true)]);
    let frames = vec![read(
        vec![
            frame(":1.1", "/org/a11y/atspi/accessible/1", 1, &[25, 1]),
            frame(":1.1", "/org/a11y/atspi/accessible/2", 1, &[25]),
        ],
        RootRead::Read,
    )];
    let (windows, outcomes, _) = gathered(&registry, |_| true, frames);
    let pass = pass_over(windows, outcomes, Vec::new(), &X11State::Unavailable, |_| false);
    assert_eq!(pass.candidates, [0]);
    assert_eq!(pass.focused, Some(0));
}

#[test]
fn two_active_candidates_without_x11_evidence_name_no_focused_window() {
    let registry = registry(vec![app(":1.1", 1, "alpha", true)]);
    let frames = vec![read(
        vec![
            frame(":1.1", "/org/a11y/atspi/accessible/1", 1, &[25, 1]),
            frame(":1.1", "/org/a11y/atspi/accessible/2", 1, &[25, 1]),
        ],
        RootRead::Read,
    )];
    let (windows, outcomes, _) = gathered(&registry, |_| true, frames);
    let pass = pass_over(windows, outcomes, Vec::new(), &X11State::Unavailable, |_| false);
    assert_eq!(pass.candidates, [0, 1]);
    assert_eq!(pass.focused, None);
}

#[test]
fn no_active_window_means_no_candidates() {
    let pass = pass_over(Vec::new(), Vec::new(), Vec::new(), &X11State::Unavailable, |_| false);
    assert!(pass.candidates.is_empty());
    assert_eq!(pass.focused, None);
}

#[test]
fn a_selected_app_with_no_outcome_is_skipped_and_one_with_an_outcome_is_not() {
    use super::{AppOutcome, Pass, skipped_selected};
    let registry = registry(vec![
        app(":1.1", 11, "calc", true),
        app(":1.2", 12, "edit", false),
    ]);
    let pass = |apps: &[usize]| Pass {
        windows: Vec::new(),
        outcomes: apps
            .iter()
            .map(|&app| AppOutcome {
                app,
                read: RootRead::Read,
            })
            .collect(),
        candidates: Vec::new(),
        focused: None,
        recovered: Vec::new(),
    };
    assert!(skipped_selected(&registry, &pass(&[0]), |_| true));
    assert!(!skipped_selected(&registry, &pass(&[0, 1]), |_| true));
    assert!(!skipped_selected(&registry, &pass(&[0]), |app| app
        .root
        .pid()
        == 11));
}

#[test]
fn a_stopped_app_the_display_server_supplied_windows_for_is_not_skipped() {
    use super::{AppOutcome, Pass, skipped_selected};
    let registry = registry(vec![
        app(":1.1", 11, "calc", true),
        app(":1.2", 12, "edit", false),
    ]);
    let pass = Pass {
        windows: Vec::new(),
        outcomes: vec![AppOutcome {
            app: 0,
            read: RootRead::Read,
        }],
        candidates: Vec::new(),
        focused: None,
        recovered: vec![1],
    };
    assert!(!skipped_selected(&registry, &pass, |_| true));
}
