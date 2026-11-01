use agent_desktop_core::{AdapterError, ErrorCode, NativeHandle};

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
    let point = Point { x: 1.0, y: 1.0 };
    assert!(
        bound_hit(&bus, &handle, &point, deadline)
            .unwrap()
            .is_none()
    );
}
