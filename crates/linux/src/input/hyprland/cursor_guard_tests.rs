use std::cell::Cell;
use std::rc::Rc;

use agent_desktop_core::{AdapterError, Deadline, DeliverySemantics, ErrorCode};

use super::*;
use crate::input::hyprland::fake_pointer_sink::{FakePointerSink, SinkEvent};
use crate::input::hyprland::pointer_sink::PointerSink;

#[derive(Default)]
struct Counts {
    hides: Cell<u32>,
    shows: Cell<u32>,
}

struct FakeHider {
    counts: Rc<Counts>,
    fail: bool,
}

impl CursorHider for FakeHider {
    fn hide(&mut self, _deadline: Deadline) -> Result<(), AdapterError> {
        self.counts.hides.set(self.counts.hides.get() + 1);
        if self.fail {
            return Err(AdapterError::new(ErrorCode::ActionFailed, "overlay lost"));
        }
        Ok(())
    }

    fn show(&mut self) {
        self.counts.shows.set(self.counts.shows.get() + 1);
    }
}

fn hider(fail: bool) -> (FakeHider, Rc<Counts>) {
    let counts = Rc::new(Counts::default());
    (
        FakeHider {
            counts: Rc::clone(&counts),
            fail,
        },
        counts,
    )
}

#[test]
fn a_failing_hide_sends_no_pointer_event() {
    let (mut hider, counts) = hider(true);
    let mut sink = FakePointerSink::default();
    let outcome = with_cursors_hidden(&mut hider, deadline(), || sink.button(0x110, true));
    let error = outcome
        .err()
        .unwrap_or_else(|| panic!("hide failure must refuse"));
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
    assert_eq!(
        error.details.unwrap_or_default()["physical_delivery_started"],
        false
    );
    assert!(sink.events.is_empty());
    assert_eq!(counts.shows.get(), 0);
}

#[test]
fn a_failing_hide_sends_no_key_event() {
    let (mut hider, _) = hider(true);
    let keys = Cell::new(0);
    let outcome = with_cursors_hidden(&mut hider, deadline(), || {
        keys.set(keys.get() + 1);
        Ok(())
    });
    assert!(outcome.is_err());
    assert_eq!(keys.get(), 0);
}

#[test]
fn show_runs_after_a_successful_send() {
    let (mut hider, counts) = hider(false);
    let mut sink = FakePointerSink::default();
    let outcome = with_cursors_hidden(&mut hider, deadline(), || sink.button(0x110, true));
    assert!(outcome.is_ok());
    assert_eq!(sink.events, vec![SinkEvent::Button(0x110, true)]);
    assert_eq!((counts.hides.get(), counts.shows.get()), (1, 1));
}

#[test]
fn show_runs_after_a_failed_send() {
    let (mut hider, counts) = hider(false);
    let outcome: Result<(), AdapterError> = with_cursors_hidden(&mut hider, deadline(), || {
        Err(AdapterError::new(ErrorCode::ActionFailed, "stopped"))
    });
    assert!(outcome.is_err());
    assert_eq!(counts.shows.get(), 1);
}

#[test]
fn show_runs_after_a_panic_in_send() {
    let (mut hider, counts) = hider(false);
    let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _: Result<(), AdapterError> =
            with_cursors_hidden(&mut hider, deadline(), || panic!("send broke"));
    }));
    assert!(unwound.is_err());
    assert_eq!(counts.shows.get(), 1);
}

fn deadline() -> Deadline {
    Deadline::after(1_000).unwrap_or_else(|error| panic!("{error:?}"))
}
