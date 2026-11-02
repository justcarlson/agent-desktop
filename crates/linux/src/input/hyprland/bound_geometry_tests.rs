use agent_desktop_core::{AdapterError, Deadline, ErrorCode, NativeHandle};

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
