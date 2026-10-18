use std::cell::Cell;
use std::time::Duration;

use agent_desktop_core::{
    AdapterError, Deadline, ObservationRequest, ObservationRoot, TreeOptions, WindowInfo,
    WindowState,
};

use super::{SettleHost, observe_with};
use crate::system::window_id;
use crate::tree::role_numbers::{DOCUMENT_WEB, PUSH_BUTTON};
use crate::tree::test_support::read_of;
use crate::tree::walker::walk_levels;
use crate::tree::wrapper::classify;
use crate::tree::{AtspiElement, Expansion, ObservedRead, Walk, WalkLimits};

fn window() -> WindowInfo {
    WindowInfo {
        id: window_id::encode(":1.7", "/org/a11y/atspi/accessible/1").expect("encodes"),
        title: "t".to_owned(),
        app: "a".to_owned(),
        pid: 77.into(),
        process_instance: Some("i".to_owned()),
        bounds: None,
        state: WindowState::default(),
    }
}

fn request(ms: u64) -> ObservationRequest {
    ObservationRequest::snapshot(
        &TreeOptions::default(),
        Deadline::after(ms).expect("deadline"),
    )
}

fn walk_of(nodes: usize, document: bool) -> Result<Walk<ObservedRead>, AdapterError> {
    let role = if document { DOCUMENT_WEB } else { PUSH_BUTTON };
    walk_levels(
        0u32,
        &WalkLimits::default(),
        Deadline::after(5_000).expect("deadline"),
        |level| {
            Ok(level
                .iter()
                .map(|key| {
                    let children = if *key == 0 {
                        (1..u32::try_from(nodes).unwrap_or(1)).collect()
                    } else {
                        Vec::new()
                    };
                    Expansion::read(read_of(role, "b", &["click"]), children)
                })
                .collect())
        },
        classify,
    )
}

struct Run {
    walks: usize,
    waited: Duration,
    complete: bool,
}

fn observe(web: bool, document: bool, ms: u64, curve: &[Result<usize, AdapterError>]) -> Run {
    let calls = Cell::new(0usize);
    let waited = Cell::new(Duration::ZERO);
    let mut walker = |_: &AtspiElement, _: &WalkLimits, _| {
        let index = calls.get().min(curve.len() - 1);
        calls.set(calls.get() + 1);
        curve[index]
            .clone()
            .and_then(|nodes| walk_of(nodes, document))
    };
    let mut pause = |gap: Duration| {
        waited.set(waited.get() + gap);
        if ms < 10_000 {
            std::thread::sleep(gap);
        }
    };
    let window = window();
    let tree = observe_with(
        ObservationRoot::Window(&window),
        &request(ms),
        SettleHost {
            walker: &mut walker,
            is_web_process: &|_| web,
            pause: &mut pause,
        },
    )
    .expect("observes");
    Run {
        walks: calls.get(),
        waited: waited.get(),
        complete: tree.is_complete(),
    }
}

fn counts(values: &[usize]) -> Vec<Result<usize, AdapterError>> {
    values.iter().map(|count| Ok(*count)).collect()
}

#[test]
fn a_web_process_is_read_again_until_three_reads_agree() {
    let run = observe(true, false, 60_000, &counts(&[18, 18, 120, 247, 247, 247]));
    assert_eq!(run.walks, 6);
    assert_eq!(run.waited, Duration::from_millis(1_000));
    assert!(run.complete);
}

#[test]
fn a_native_app_is_read_once_and_never_waited_for() {
    let run = observe(false, false, 60_000, &counts(&[18, 120]));
    assert_eq!(run.walks, 1);
    assert_eq!(run.waited, Duration::ZERO);
    assert!(run.complete);
}

#[test]
fn a_web_document_in_a_native_binary_triggers_the_wait() {
    let run = observe(false, true, 60_000, &counts(&[40, 40, 40]));
    assert_eq!(run.walks, 3);
}

#[test]
fn a_tree_still_growing_when_the_budget_ends_is_returned_incomplete() {
    let run = observe(
        true,
        false,
        1_000,
        &counts(&[18, 40, 80, 120, 160, 200, 240]),
    );
    assert!(!run.complete);
    assert!(run.waited <= Duration::from_millis(700));
    assert!(run.walks < 7);
}

#[test]
fn a_budget_smaller_than_one_poll_returns_the_first_tree_incomplete() {
    let run = observe(true, false, 200, &counts(&[18]));
    assert_eq!(run.walks, 1);
    assert!(!run.complete);
}

#[test]
fn a_poll_that_runs_out_of_budget_returns_the_walk_before_it_incomplete() {
    let curve = vec![Ok(18), Ok(40), Err(AdapterError::timeout("slow"))];
    let run = observe(true, false, 60_000, &curve);
    assert_eq!(run.walks, 3);
    assert!(!run.complete);
}

#[test]
fn a_poll_that_finds_the_window_gone_is_window_not_found() {
    let curve = [
        Ok(18),
        Err(AdapterError::new(
            agent_desktop_core::ErrorCode::ElementNotFound,
            "gone",
        )),
    ];
    let calls = Cell::new(0usize);
    let mut walker = |_: &AtspiElement, _: &WalkLimits, _| {
        let index = calls.get().min(curve.len() - 1);
        calls.set(calls.get() + 1);
        curve[index].clone().and_then(|nodes| walk_of(nodes, false))
    };
    let window = window();
    let error = observe_with(
        ObservationRoot::Window(&window),
        &request(60_000),
        SettleHost {
            walker: &mut walker,
            is_web_process: &|_| true,
            pause: &mut |_| {},
        },
    )
    .expect_err("gone");
    assert_eq!(error.code, agent_desktop_core::ErrorCode::WindowNotFound);
}
