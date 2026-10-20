use agent_desktop_core::{AdapterError, Deadline, ErrorCode, NativeHandle};

use super::*;
use crate::tree::frame_geometry::pointer_bounds;
use crate::tree::node_evidence::evidence_for;
use crate::tree::role_numbers::PUSH_BUTTON;
use crate::tree::test_support::{read_of, words};
use crate::tree::{AtspiElement, Extents, Field, ancestry};

struct Counting {
    answer: Result<(Liveness, ()), AdapterError>,
    reads: usize,
    stopped: Option<String>,
    stopped_asked: usize,
    placements: usize,
    global_asked: usize,
    global: bool,
    placed: Placement,
    states: Option<Vec<u32>>,
    at: Option<Extents>,
}

impl Counting {
    fn alive() -> Self {
        Self {
            answer: Ok((Liveness::Alive, ())),
            reads: 0,
            stopped: None,
            stopped_asked: 0,
            placements: 0,
            global_asked: 0,
            global: true,
            placed: Placement::default(),
            states: None,
            at: None,
        }
    }
}

fn deadline() -> Deadline {
    Deadline::after(5_000).expect("a deadline builds")
}

fn rect(x: i32, y: i32, width: i32, height: i32) -> Extents {
    Extents {
        x,
        y,
        width,
        height,
    }
}

impl NodeReader for Counting {
    fn read(&mut self, element: &AtspiElement) -> Result<NodeEvidence, AdapterError> {
        self.reads += 1;
        let (liveness, ()) = self.answer.clone()?;
        let mut read = read_of(PUSH_BUTTON, "Save", &["click"]);
        if let Some(bits) = &self.states {
            read.node.traits.states = Field::Present(bits.clone());
        }
        if let Some(at) = self.at {
            read.node.extents = Field::Present(at);
        }
        Ok(NodeEvidence {
            evidence: evidence_for(&read, element),
            read,
            liveness,
        })
    }

    fn stopped_app(&mut self, _pid: u32) -> Option<String> {
        self.stopped_asked += 1;
        self.stopped.clone()
    }

    fn placement(&mut self, _element: &AtspiElement, _deadline: Deadline) -> Placement {
        self.placements += 1;
        self.placed.clone()
    }

    fn global_position(&mut self, _element: &AtspiElement, _deadline: Deadline) -> bool {
        self.global_asked += 1;
        self.global
    }
}

fn handle() -> NativeHandle {
    let mut element = AtspiElement::new(":1.1", "/p", 40);
    element.set_ancestry(ancestry::of_root(&element.clone()));
    element.into_native_handle()
}

fn kind(error: &AdapterError) -> &str {
    error
        .details
        .as_ref()
        .and_then(|details| details.get("kind"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
}

#[test]
fn every_projection_of_a_node_costs_exactly_one_node_read() {
    let mut reader = Counting::alive();
    let handle = handle();
    value_via(&mut reader, &handle).expect("value");
    assert_eq!(reader.reads, 1);
    state_via(&mut reader, &handle, deadline()).expect("state");
    assert_eq!(reader.reads, 2);
    actions_via(&mut reader, &handle).expect("actions");
    assert_eq!(reader.reads, 3);
    element_via(&mut reader, &handle, deadline()).expect("element");
    assert_eq!(reader.reads, 4);
    bounds_via(&mut reader, &handle, deadline()).expect("bounds");
    assert_eq!(reader.reads, 5);
}

#[test]
fn each_projection_answers_with_its_own_part_of_the_one_read() {
    let mut reader = Counting::alive();
    let handle = handle();
    let element = element_via(&mut reader, &handle, deadline()).expect("element");
    let actions = actions_via(&mut reader, &handle)
        .expect("actions")
        .expect("listed");
    assert_eq!(actions, element.available_actions);
    let state = state_via(&mut reader, &handle, deadline())
        .expect("state")
        .expect("read");
    assert_eq!(state.role, element.state.role);
}

#[test]
fn a_destroyed_object_and_an_exited_application_are_settled_stale_refs() {
    for (answer, expected) in [
        (
            Err(AdapterError::new(ErrorCode::ElementNotFound, "gone")),
            "resolve_target_destroyed",
        ),
        (
            Err(AdapterError::new(ErrorCode::AppNotFound, "left")),
            "resolve_owning_process_exited",
        ),
        (Ok((Liveness::ObjectGone, ())), "resolve_target_destroyed"),
        (Ok((Liveness::AppGone, ())), "resolve_owning_process_exited"),
    ] {
        let mut reader = Counting::alive();
        reader.answer = answer;
        let error = read_live(&mut reader, &handle()).expect_err("stale");
        assert_eq!(error.code, ErrorCode::StaleRef);
        assert_eq!(kind(&error), expected);
        assert!(!error.is_explicitly_retryable());
        assert_eq!(reader.stopped_asked, 0);
    }
}

#[test]
fn a_node_that_did_not_answer_is_a_timeout_and_unresponsive_when_its_process_is_stopped() {
    let mut reader = Counting::alive();
    reader.answer = Ok((Liveness::TimedOut, ()));
    let error = read_live(&mut reader, &handle()).expect_err("timeout");
    assert_eq!(error.code, ErrorCode::Timeout);

    reader.stopped = Some("Calculator".to_owned());
    let error = read_live(&mut reader, &handle()).expect_err("stopped");
    assert_eq!(error.code, ErrorCode::AppUnresponsive);

    let mut slow = Counting::alive();
    slow.answer = Err(AdapterError::timeout("deadline"));
    assert_eq!(
        read_live(&mut slow, &handle()).expect_err("timeout").code,
        ErrorCode::Timeout
    );
}

#[test]
fn a_handle_from_another_platform_or_an_empty_one_is_invalid_args() {
    let mut reader = Counting::alive();
    for foreign in [NativeHandle::new(5u32), NativeHandle::null()] {
        let error = read_live(&mut reader, &foreign).expect_err("invalid");
        assert_eq!(error.code, ErrorCode::InvalidArgs);
        assert_eq!(kind(&error), "invalid_native_handle");
    }
    assert_eq!(reader.reads, 0);
}

fn on_screen_bits() -> Vec<u32> {
    words(&[8, 25, 30])
}

#[test]
fn a_state_read_costs_the_node_read_plus_one_placement_never_one_per_field() {
    let mut reader = Counting::alive();
    reader.states = Some(on_screen_bits());
    state_via(&mut reader, &handle(), deadline()).expect("state");
    assert_eq!((reader.reads, reader.placements), (1, 1));
    element_via(&mut reader, &handle(), deadline()).expect("element");
    assert_eq!((reader.reads, reader.placements), (2, 2));
}

#[test]
fn a_node_the_read_alone_settles_costs_no_placement() {
    let mut reader = Counting::alive();
    reader.states = Some(words(&[8, 30]));
    reader.at = Some(rect(i32::MIN, i32::MIN, 420, 22));
    let state = state_via(&mut reader, &handle(), deadline())
        .expect("state")
        .expect("read");
    assert_eq!(state.offscreen, Some(true));
    assert_eq!(reader.placements, 0);
}

#[test]
fn bounds_of_a_global_toolkit_are_as_read_with_no_placement() {
    let mut reader = Counting::alive();
    reader.at = Some(rect(100, 137, 420, 34));
    let bounds = bounds_via(&mut reader, &handle(), deadline())
        .expect("bounds")
        .expect("some");
    assert_eq!((bounds.x, bounds.y), (100.0, 137.0));
    assert_eq!((reader.placements, reader.global_asked), (0, 1));
}

#[test]
fn bounds_of_another_toolkit_are_the_pointer_rule_over_the_placement_and_never_the_raw_extents() {
    let mut reader = Counting::alive();
    reader.global = false;
    reader.at = Some(rect(0, 0, 498, 34));
    reader.placed = Placement {
        node: Some(rect(0, 0, 498, 34)),
        toolkit: crate::system::toolkit_id::ToolkitId {
            name: Some("GTK".to_owned()),
            version: Some("4.14.5".to_owned()),
        },
        x11: Some(crate::system::x11::window_x11_state::WindowX11State {
            shadow: (0, 0, 0, 0),
            client_id: 7,
            hidden: false,
            client_origin: (302, 260),
        }),
        ..Placement::default()
    };
    let expected = pointer_bounds(
        &reader.placed.toolkit,
        rect(0, 0, 498, 34),
        rect(0, 0, 498, 34),
        Some((302, 260)),
    )
    .map(Rect::from);
    let bounds = bounds_via(&mut reader, &handle(), deadline()).expect("bounds");
    assert_eq!(bounds, expected);
    assert_eq!(reader.placements, 1);
    reader.placed.node = None;
    assert_eq!(
        bounds_via(&mut reader, &handle(), deadline()).expect("bounds"),
        None
    );
}

#[test]
fn a_live_element_of_another_toolkit_has_no_bounds_when_the_position_is_unprovable() {
    let mut reader = Counting::alive();
    reader.global = false;
    reader.at = Some(rect(5, 6, 498, 34));
    let element = element_via(&mut reader, &handle(), deadline()).expect("element");
    assert_eq!(element.bounds, None);
}
