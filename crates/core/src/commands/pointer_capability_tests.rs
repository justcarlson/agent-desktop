use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;
use crate::adapter::{ActionOps, InputOps, ObservationOps, SystemOps};
use crate::{
    AdapterError, CommandContext, DeliverySemantics, ErrorCode, InteractionLease, MouseEvent,
};
use serde_json::json;

struct NoPointerAdapter {
    sent: AtomicUsize,
}

impl NoPointerAdapter {
    fn new() -> Self {
        Self {
            sent: AtomicUsize::new(0),
        }
    }
}

impl ObservationOps for NoPointerAdapter {}

impl ActionOps for NoPointerAdapter {}

impl InputOps for NoPointerAdapter {
    fn physical_pointer_refusal(&self) -> Option<AdapterError> {
        Some(
            AdapterError::new(
                ErrorCode::ActionNotSupported,
                "Pointer input is not available in this session",
            )
            .with_details(json!({
                "cause": "wayland_session",
                "physical_delivery_started": false,
            }))
            .with_disposition(DeliverySemantics::not_delivered()),
        )
    }

    fn mouse_event(
        &self,
        _event: MouseEvent,
        _lease: &InteractionLease,
    ) -> Result<(), AdapterError> {
        self.sent.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

impl SystemOps for NoPointerAdapter {}

#[test]
fn a_headed_hover_is_refused_before_the_cursor_is_ever_moved() {
    let adapter = NoPointerAdapter::new();
    let error = hover::execute(
        hover::HoverArgs {
            ref_id: None,
            xy: Some((10.0, 10.0)),
            duration_ms: None,
            timeout_ms: None,
        },
        &adapter,
        &CommandContext::default().with_headed(true),
    )
    .expect_err("a session without pointer input must refuse the hover");

    assert_eq!(error.code(), "ACTION_NOT_SUPPORTED");
    assert_eq!(error.code(), "ACTION_NOT_SUPPORTED");
    assert_eq!(adapter.sent.load(Ordering::SeqCst), 0);
}

#[test]
fn a_headed_mouse_move_is_refused_before_the_cursor_is_ever_moved() {
    let adapter = NoPointerAdapter::new();
    let error = mouse_move::execute(
        mouse_move::MouseMoveArgs { x: 10.0, y: 10.0 },
        &adapter,
        &CommandContext::default().with_headed(true),
    )
    .expect_err("a session without pointer input must refuse the move");

    assert_eq!(error.code(), "ACTION_NOT_SUPPORTED");
    assert_eq!(adapter.sent.load(Ordering::SeqCst), 0);
}
