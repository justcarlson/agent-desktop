use std::cell::{Cell, RefCell};

use agent_desktop_core::{AdapterError, DeliverySemantics, ErrorCode};

use super::*;
use crate::actions::chain::DeliveryOutcome;

struct Control {
    maximum: f64,
    spin_step: Option<f64>,
    reply: WriteReply,
    held: RefCell<f64>,
    sets: Cell<usize>,
    readable: bool,
}

impl Control {
    fn slider() -> Self {
        Self {
            maximum: 100.0,
            spin_step: None,
            reply: WriteReply::Done,
            held: RefCell::new(0.0),
            sets: Cell::new(0),
            readable: true,
        }
    }
}

impl NumericPort for Control {
    fn set(&self, value: f64) -> Result<WriteReply, AdapterError> {
        self.sets.set(self.sets.get() + 1);
        if self.reply == WriteReply::Done {
            let stored = match self.spin_step {
                Some(step) if step >= 1.0 => value.round(),
                _ => value,
            };
            *self.held.borrow_mut() = stored.min(self.maximum);
        }
        Ok(self.reply)
    }

    fn stored(&self) -> Option<f64> {
        self.readable.then(|| *self.held.borrow())
    }
}

fn write(control: &Control, plan: Plan) -> Result<DeliveryOutcome, AdapterError> {
    let mut rungs = write_rungs(control, plan);
    (rungs[0].run)()
}

fn planned(request: &str, spin: bool, step: Option<f64>) -> Plan {
    plan(request, spin, step).expect("a valid plan")
}

#[test]
fn a_value_in_range_reads_back_and_is_matched() {
    let control = Control::slider();
    let outcome = write(&control, planned("0.2", false, None)).expect("written");
    assert_eq!(outcome, DeliveryOutcome::Delivered(Observed::Matched));
}

#[test]
fn a_value_above_the_range_reads_back_clamped_and_is_a_mismatch() {
    let control = Control::slider();
    let outcome = write(&control, planned("500", false, None)).expect("written");
    assert_eq!(outcome, DeliveryOutcome::Delivered(Observed::Unchanged));
    assert_eq!(*control.held.borrow(), 100.0);
}

#[test]
fn a_scroll_bar_written_to_its_advertised_maximum_reports_the_lower_reachable_one() {
    let mut control = Control::slider();
    control.maximum = 9961.0;
    let outcome = write(&control, planned("10419", false, None)).expect("written");
    assert_eq!(outcome, DeliveryOutcome::Delivered(Observed::Unchanged));
}

#[test]
fn a_whole_step_spin_button_refuses_a_fraction_before_any_write() {
    let control = Control::slider();
    let error = plan("7.5", true, Some(1.0)).expect_err("refused");
    assert_eq!(error.code, ErrorCode::InvalidArgs);
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
    assert_eq!(control.sets.get(), 0);
}

#[test]
fn a_whole_step_spin_button_takes_a_whole_number() {
    assert!(plan("7", true, Some(1.0)).is_ok());
    assert!(plan("7.0", true, Some(1.0)).is_ok());
}

#[test]
fn a_fractional_step_spin_button_takes_a_fraction() {
    let plan = planned("0.25", true, Some(0.05));
    assert!(plan.readback_proves_commit);
}

#[test]
fn a_slider_takes_a_fraction_whatever_its_step() {
    assert!(plan("7.5", false, Some(1.0)).is_ok());
}

#[test]
fn a_fraction_on_a_spin_button_with_an_unreadable_step_is_not_proved_by_the_read_back() {
    let control = Control::slider();
    let plan = planned("7.5", true, None);
    assert!(!plan.readback_proves_commit);
    let outcome = write(&control, plan).expect("written");
    assert_eq!(outcome, DeliveryOutcome::Delivered(Observed::Unread));
}

#[test]
fn a_whole_number_on_a_spin_button_with_an_unreadable_step_is_still_proved() {
    assert!(planned("7", true, None).readback_proves_commit);
}

#[test]
fn a_non_numeric_or_non_finite_request_is_invalid_args() {
    for request in ["abc", "", "NaN", "inf", "-inf", "1e999", "7,5"] {
        let error = plan(request, false, None).expect_err(request);
        assert_eq!(error.code, ErrorCode::InvalidArgs, "{request}");
        assert_eq!(error.disposition, DeliverySemantics::not_delivered());
    }
}

#[test]
fn a_request_with_spaces_around_it_is_read() {
    assert_eq!(planned("  42 ", false, None).value, 42.0);
}

#[test]
fn a_read_only_answer_is_action_not_supported_and_not_delivered() {
    let mut control = Control::slider();
    control.reply = WriteReply::Absent;
    let error = write(&control, planned("5", false, None)).expect_err("refused");
    assert_eq!(error.code, ErrorCode::ActionNotSupported);
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
}

#[test]
fn an_unreadable_value_after_the_write_is_unobserved() {
    let mut control = Control::slider();
    control.readable = false;
    let outcome = write(&control, planned("5", false, None)).expect("written");
    assert_eq!(outcome, DeliveryOutcome::Delivered(Observed::Unread));
}

#[test]
fn agreement_tolerates_float_noise_but_not_a_real_difference() {
    assert!(agrees(0.1 + 0.2, 0.3));
    assert!(!agrees(100.0, 500.0));
}

#[test]
fn a_zero_step_is_an_unreadable_step() {
    let plan = planned("7.5", true, Some(0.0));
    assert!(!plan.readback_proves_commit);
}

#[test]
fn a_value_that_stays_at_its_old_reading_is_write_ignored_and_not_delivered() {
    let mut control = Control::slider();
    control.reply = WriteReply::Done;
    control.maximum = 0.0;
    let error = write(&control, planned("5", false, None)).expect_err("ignored");
    assert_eq!(error.code, ErrorCode::ActionFailed);
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
    let cause = error.details.as_ref().and_then(|d| d.get("cause"));
    assert_eq!(cause.and_then(|c| c.as_str()), Some("write_ignored"));
}
