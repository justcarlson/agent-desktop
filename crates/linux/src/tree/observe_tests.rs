use agent_desktop_core::{
    AdapterError, Deadline, ErrorCode, ObservationRequest, ObservationRoot, SnapshotSurface,
    TreeOptions, WindowInfo, WindowState,
};

use super::{SettleHost, observe_with};
use crate::system::window_id;
use crate::tree::role_numbers::PUSH_BUTTON;
use crate::tree::test_support::read_of;
use crate::tree::walker::walk_levels;
use crate::tree::wrapper::classify;
use crate::tree::{AtspiElement, Expansion, ObservedRead, Walk, WalkLimits};

pub(super) fn observe_plain(
    root: ObservationRoot<'_>,
    request: &ObservationRequest,
    mut walker: impl FnMut(
        &AtspiElement,
        &WalkLimits,
        Deadline,
    ) -> Result<Walk<ObservedRead>, AdapterError>,
) -> Result<agent_desktop_core::ObservedTree, AdapterError> {
    observe_with(
        root,
        request,
        SettleHost {
            walker: &mut walker,
            is_web_process: &|_| false,
            pause: &mut |_| {},
        },
    )
}

fn window(id: String) -> WindowInfo {
    WindowInfo {
        id,
        title: "t".to_owned(),
        app: "a".to_owned(),
        pid: 77.into(),
        process_instance: Some("i".to_owned()),
        bounds: None,
        state: WindowState::default(),
    }
}

fn request() -> ObservationRequest {
    ObservationRequest::snapshot(
        &TreeOptions::default(),
        Deadline::after(5_000).expect("deadline"),
    )
}

fn one_node_walk(limits: &WalkLimits) -> Result<Walk<ObservedRead>, AdapterError> {
    walk_levels(
        0u32,
        limits,
        Deadline::after(5_000).expect("deadline"),
        |level| {
            Ok(level
                .iter()
                .map(|_| Expansion::read(read_of(PUSH_BUTTON, "b", &["click"]), Vec::new()))
                .collect())
        },
        classify,
    )
}

#[test]
fn a_window_id_decodes_to_the_element_the_walk_starts_from() {
    let id = window_id::encode(":1.7", "/org/a11y/atspi/accessible/1").expect("encodes");
    let window = window(id);
    let mut seen = None;
    let tree = observe_plain(
        ObservationRoot::Window(&window),
        &request(),
        |element, limits, _| {
            seen = Some(element.clone());
            one_node_walk(limits)
        },
    )
    .expect("observes");
    assert!(tree.is_complete());
    assert_eq!(
        seen,
        Some(AtspiElement::new(
            ":1.7",
            "/org/a11y/atspi/accessible/1",
            77
        ))
    );
}

#[test]
fn a_foreign_window_id_is_invalid_args_and_nothing_is_walked() {
    let window = window("w-12345".to_owned());
    let error = observe_plain(ObservationRoot::Window(&window), &request(), |_, _, _| {
        panic!("the walk must not start")
    })
    .expect_err("rejected");
    assert_eq!(error.code, ErrorCode::InvalidArgs);
}

#[test]
fn a_surface_this_adapter_does_not_walk_is_not_supported() {
    let id = window_id::encode(":1.7", "/a").expect("encodes");
    let window = window(id);
    let mut menu = request();
    menu.surface = SnapshotSurface::Menu;
    let error = observe_plain(ObservationRoot::Window(&window), &menu, |_, _, _| {
        panic!("the walk must not start")
    })
    .expect_err("rejected");
    assert_eq!(error.code, ErrorCode::PlatformNotSupported);
}

#[test]
fn a_window_that_left_the_bus_is_window_not_found() {
    let id = window_id::encode(":1.7", "/a").expect("encodes");
    let window = window(id);
    for code in [ErrorCode::AppNotFound, ErrorCode::ElementNotFound] {
        let error = observe_plain(ObservationRoot::Window(&window), &request(), |_, _, _| {
            Err(AdapterError::new(code.clone(), "gone"))
        })
        .expect_err("gone");
        assert_eq!(error.code, ErrorCode::WindowNotFound);
    }
}

#[test]
fn a_timeout_is_not_rewritten() {
    let id = window_id::encode(":1.7", "/a").expect("encodes");
    let window = window(id);
    let error = observe_plain(ObservationRoot::Window(&window), &request(), |_, _, _| {
        Err(AdapterError::timeout("slow"))
    })
    .expect_err("slow");
    assert_eq!(error.code, ErrorCode::Timeout);
}

#[test]
fn an_incomplete_walk_is_a_success_that_the_all_or_nothing_read_refuses() {
    let id = window_id::encode(":1.7", "/a").expect("encodes");
    let window = window(id);
    let tree = observe_plain(
        ObservationRoot::Window(&window),
        &request(),
        |_, limits, _| {
            let mut walk = one_node_walk(limits)?;
            walk.complete = false;
            walk.root.truncated = true;
            Ok(walk)
        },
    )
    .expect("a cut walk still observes");
    assert!(!tree.is_complete());
    assert_eq!(
        tree.into_accessibility_tree().expect_err("incomplete").code,
        ErrorCode::Timeout
    );
}

#[test]
fn a_request_deeper_than_the_raw_limit_is_rejected_before_any_walk() {
    let id = window_id::encode(":1.7", "/a").expect("encodes");
    let window = window(id);
    let mut bad = request();
    bad.max_logical_depth = 60;
    let error = observe_plain(ObservationRoot::Window(&window), &bad, |_, _, _| {
        panic!("the walk must not start")
    })
    .expect_err("rejected");
    assert_eq!(error.code, ErrorCode::InvalidArgs);
}
