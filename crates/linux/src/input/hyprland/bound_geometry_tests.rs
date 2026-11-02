use agent_desktop_core::tree::state::{HIDDEN, OFFSCREEN};
use agent_desktop_core::{AdapterError, Deadline, ErrorCode, LiveElement, NativeHandle, Rect};

use super::*;

#[test]
fn a_timeout_while_placing_stays_an_error() {
    let timed_out: Result<Option<u8>, AdapterError> =
        Err(AdapterError::new(ErrorCode::Timeout, "deadline"));
    let error = unplaced_as(timed_out, None).unwrap_err();
    assert_eq!(error.code, ErrorCode::Timeout);
}

#[test]
fn a_refusal_to_place_answers_unplaced() {
    let refused: Result<Option<u8>, AdapterError> = Err(
        crate::system::hyprland::receiver::receiver_refusal::ReceiverRefusal::EvidenceMissing {
            check: "frame_title",
        }
        .to_error(),
    );
    assert_eq!(unplaced_as(refused, None).unwrap(), None);
}

#[test]
fn no_bound_hyprland_leaves_bounds_and_hits_to_the_toolkit_path() {
    let handle = NativeHandle::null();
    let bus = SharedBus::default();
    let deadline = agent_desktop_core::Deadline::after(1_000).unwrap();
    assert!(bound_bounds(&bus, &handle, deadline).unwrap().is_none());
    assert!(bound_position_refusal(&bus, &handle, deadline).is_none());
    let point = Point { x: 1.0, y: 1.0 };
    assert!(
        bound_hit(&bus, &handle, &point, deadline)
            .unwrap()
            .is_none()
    );
}

struct CountingHider {
    hides: u32,
}

impl CursorHider for CountingHider {
    fn hide(&mut self, _deadline: Deadline) -> Result<(), AdapterError> {
        self.hides += 1;
        Ok(())
    }

    fn show(&mut self) {}
}

#[test]
fn a_drag_endpoint_on_an_xwayland_target_with_a_stack_proof_is_refused_before_any_event() {
    use crate::input::hyprland::ref_pointer::tests::{
        assert_unproven, placed, proven_xwayland_stack,
    };

    let fake = proven_xwayland_stack("xwayland-drag");
    let mut hider = CountingHider { hides: 0 };
    let deadline = Deadline::detached_after(3_000).unwrap();
    let error = prove_placed_point(&fake.binding, &placed(), &mut hider, deadline)
        .expect_err("a stack proof is not receiver proof on XWayland");
    assert_unproven(&error);
    assert_eq!(hider.hides, 0, "the refusal comes before the cursors hide");
}

fn read_live(role: &str, state: &str, toolkit: Option<Rect>) -> LiveElement {
    use agent_desktop_core::{ElementState, IdentifierEvidence, LiveIdentity, LocatorField};
    LiveElement {
        identity: LiveIdentity {
            name: LocatorField::Known("zoo".to_owned()),
            description: LocatorField::Absent,
            identifiers: IdentifierEvidence::default(),
        },
        state: ElementState {
            role: role.to_owned(),
            states: vec![state.to_owned()],
            value: None,
            enabled: Some(true),
            hidden: None,
            offscreen: None,
        },
        states_complete: true,
        bounds: toolkit,
        available_actions: vec!["Click".to_owned()],
    }
}

fn rect(x: f64, y: f64, width: f64, height: f64) -> Rect {
    Rect {
        x,
        y,
        width,
        height,
    }
}

#[test]
fn a_popup_menu_item_the_compositor_cannot_place_keeps_its_visibility_evidence() {
    let read = read_live("menuitem", "selected", Some(rect(4.0, 4.0, 120.0, 24.0)));
    let live = with_bound_bounds(read, None);
    assert_eq!(
        live.bounds, None,
        "popup-local extents are no pointer position"
    );
    assert_eq!(live.state.offscreen, Some(false));
}

#[test]
fn a_frameless_xwayland_toplevel_the_compositor_cannot_place_keeps_its_visibility_evidence() {
    let read = read_live("button", "focused", Some(rect(647.0, 38.0, 621.0, 750.0)));
    let live = with_bound_bounds(read, None);
    assert_eq!(
        live.bounds, None,
        "an unbound toplevel has no layout position"
    );
    assert_eq!(live.state.offscreen, Some(false));
}

#[test]
fn an_element_the_compositor_places_takes_the_placed_rectangle_and_no_visibility_claim() {
    let placed = rect(10.0, 20.0, 30.0, 40.0);
    let read = read_live("button", "focused", Some(rect(0.0, 0.0, 30.0, 40.0)));
    let live = with_bound_bounds(read, Some(placed));
    assert_eq!(live.bounds, Some(placed));
    assert_eq!(live.state.offscreen, None);
}

#[test]
fn an_unplaced_element_without_shown_toolkit_evidence_gains_no_visibility() {
    let toolkit = Some(rect(4.0, 4.0, 120.0, 24.0));
    let mut offscreen = read_live("menuitem", "selected", toolkit);
    offscreen.state.offscreen = Some(true);
    let mut hidden = read_live("menuitem", "selected", toolkit);
    hidden.state.hidden = Some(true);
    let cases = [
        (offscreen, Some(true)),
        (hidden, None),
        (read_live("menuitem", HIDDEN, toolkit), None),
        (read_live("menuitem", OFFSCREEN, toolkit), None),
        (read_live("menuitem", "selected", None), None),
        (
            read_live("menuitem", "selected", Some(rect(4.0, 4.0, 0.0, 24.0))),
            None,
        ),
    ];
    for (read, expected) in cases {
        let live = with_bound_bounds(read, None);
        assert_eq!(live.bounds, None);
        assert_eq!(live.state.offscreen, expected, "{:?}", live.state);
    }
}
