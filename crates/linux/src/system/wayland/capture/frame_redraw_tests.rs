use std::cell::RefCell;
use std::collections::VecDeque;

use super::*;
use crate::system::wayland::wayland_fault::WaylandFault;

fn frame(fill: u8) -> RawFrame {
    RawFrame {
        data: vec![fill; 16],
        format: 1,
        width: 2,
        height: 2,
        stride: 8,
        inverted: false,
    }
}

const REFUSED: Result<RawFrame, CaptureFault> =
    Err(CaptureFault::Failed(FrameFailure::BufferConstraints));

struct Script {
    answers: RefCell<VecDeque<Result<RawFrame, CaptureFault>>>,
    sessions: RefCell<usize>,
    pauses: RefCell<Vec<Duration>>,
}

impl Script {
    fn new(answers: Vec<Result<RawFrame, CaptureFault>>) -> Self {
        Self {
            answers: RefCell::new(answers.into()),
            sessions: RefCell::new(0),
            pauses: RefCell::new(Vec::new()),
        }
    }

    fn run(&self, budget_ms: u64) -> Result<RawFrame, CaptureFault> {
        self.run_as(Composition::Stale, budget_ms)
    }

    fn run_as(&self, composition: Composition, budget_ms: u64) -> Result<RawFrame, CaptureFault> {
        let deadline = Deadline::detached_after(budget_ms).unwrap();
        capture_fresh(
            deadline,
            composition,
            |_| {
                *self.sessions.borrow_mut() += 1;
                self.answers
                    .borrow_mut()
                    .pop_front()
                    .unwrap_or(Err(CaptureFault::FrameSilent))
            },
            |pause| self.pauses.borrow_mut().push(pause),
        )
    }
}

#[test]
fn a_current_toplevel_is_answered_with_its_first_frame() {
    let script = Script::new(vec![Ok(frame(1)), Ok(frame(2))]);
    assert_eq!(script.run_as(Composition::Current, 5000), Ok(frame(1)));
    assert_eq!(*script.sessions.borrow(), 1);
    assert!(script.pauses.borrow().is_empty());
}

#[test]
fn a_current_toplevel_is_answered_on_a_budget_too_short_for_the_redraw_wait() {
    let script = Script::new(vec![Ok(frame(1)), Ok(frame(2))]);
    assert_eq!(script.run_as(Composition::Current, 90), Ok(frame(1)));
}

#[test]
fn a_current_toplevel_with_a_refused_buffer_answers_with_the_first_frame_after_it() {
    let script = Script::new(vec![REFUSED, Ok(frame(1)), Ok(frame(2))]);
    assert_eq!(script.run_as(Composition::Current, 5000), Ok(frame(1)));
    assert_eq!(*script.sessions.borrow(), 2);
    assert_eq!(*script.pauses.borrow(), vec![CONSTRAINTS_PAUSE]);
}

#[test]
fn the_answer_is_the_first_later_frame_whose_pixels_changed() {
    let script = Script::new(vec![Ok(frame(1)), Ok(frame(2))]);
    assert_eq!(script.run(5000), Ok(frame(2)));
    assert_eq!(*script.sessions.borrow(), 2);
    assert_eq!(*script.pauses.borrow(), vec![REDRAW_SETTLE]);
}

#[test]
fn a_redraw_that_lands_after_the_first_pause_is_waited_for() {
    let script = Script::new(vec![
        Ok(frame(1)),
        Ok(frame(1)),
        Ok(frame(1)),
        Ok(frame(1)),
        Ok(frame(2)),
        Ok(frame(3)),
    ]);
    assert_eq!(script.run(5000), Ok(frame(2)));
    assert_eq!(*script.sessions.borrow(), 5);
    assert_eq!(*script.pauses.borrow(), vec![REDRAW_SETTLE; 4]);
}

#[test]
fn an_unchanged_window_is_answered_only_after_the_whole_redraw_wait() {
    let script = Script::new(vec![Ok(frame(1)); REDRAW_POLLS + 5]);
    assert_eq!(script.run(5000), Ok(frame(1)));
    assert_eq!(*script.sessions.borrow(), REDRAW_POLLS + 1);
    assert_eq!(*script.pauses.borrow(), vec![REDRAW_SETTLE; REDRAW_POLLS]);
}

#[test]
fn a_redraw_wait_the_deadline_cuts_short_times_out_instead_of_answering() {
    let script = Script::new(vec![Ok(frame(1)), Ok(frame(1)), Ok(frame(2))]);
    assert_eq!(script.run(90), Err(CaptureFault::FrameSilent));
    assert_eq!(*script.sessions.borrow(), 1);
}

#[test]
fn a_resized_frame_is_a_redraw() {
    let mut resized = frame(1);
    resized.width = 4;
    resized.stride = 16;
    let script = Script::new(vec![Ok(frame(1)), Ok(resized.clone())]);
    assert_eq!(script.run(5000), Ok(resized));
}

#[test]
fn a_missing_redraw_frame_times_out_and_never_returns_the_first_frame() {
    let script = Script::new(vec![Ok(frame(1)), Err(CaptureFault::FrameSilent)]);
    assert_eq!(script.run(5000), Err(CaptureFault::FrameSilent));
}

#[test]
fn a_session_that_goes_quiet_after_the_first_frame_is_a_missed_frame() {
    let script = Script::new(vec![
        Ok(frame(1)),
        Err(CaptureFault::Setup(WaylandFault::Silent)),
    ]);
    assert_eq!(script.run(5000), Err(CaptureFault::FrameSilent));
}

#[test]
fn a_quiet_compositor_before_any_frame_is_still_a_setup_fault() {
    let script = Script::new(vec![Err(CaptureFault::Setup(WaylandFault::Silent))]);
    assert_eq!(
        script.run(5000),
        Err(CaptureFault::Setup(WaylandFault::Silent))
    );
}

#[test]
fn a_refused_buffer_is_retried_on_a_new_session_after_a_pause() {
    let script = Script::new(vec![REFUSED, Ok(frame(1)), REFUSED, REFUSED, Ok(frame(2))]);
    assert_eq!(script.run(5000), Ok(frame(2)));
    assert_eq!(*script.sessions.borrow(), 5);
    assert_eq!(
        *script.pauses.borrow(),
        vec![
            CONSTRAINTS_PAUSE,
            REDRAW_SETTLE,
            CONSTRAINTS_PAUSE,
            CONSTRAINTS_PAUSE
        ]
    );
}

#[test]
fn refused_buffers_are_retried_a_bounded_number_of_times() {
    let script = Script::new(vec![REFUSED; CONSTRAINTS_RETRIES + 5]);
    assert_eq!(script.run(60_000), REFUSED);
    assert_eq!(*script.sessions.borrow(), CONSTRAINTS_RETRIES + 1);
}

#[test]
fn a_refused_buffer_without_time_for_the_pause_times_out() {
    let script = Script::new(vec![REFUSED, Ok(frame(1))]);
    assert_eq!(script.run(1), Err(CaptureFault::FrameSilent));
    assert_eq!(*script.sessions.borrow(), 1);
}

#[test]
fn any_other_failure_ends_the_capture_at_once() {
    for fault in [
        CaptureFault::FrameGone,
        CaptureFault::Stale,
        CaptureFault::NoFormat,
        CaptureFault::Failed(FrameFailure::Stopped),
        CaptureFault::Failed(FrameFailure::Unknown),
    ] {
        let first = Script::new(vec![Err(fault.clone()), Ok(frame(1)), Ok(frame(2))]);
        assert_eq!(first.run(5000), Err(fault.clone()));
        assert_eq!(*first.sessions.borrow(), 1);
        let second = Script::new(vec![Ok(frame(1)), Err(fault.clone()), Ok(frame(2))]);
        assert_eq!(second.run(5000), Err(fault.clone()));
        assert_eq!(*second.sessions.borrow(), 2);
    }
}

#[test]
fn the_redraw_wait_is_the_settle_window_when_time_is_plentiful() {
    assert_eq!(redraw_wait(Duration::from_secs(5)), REDRAW_SETTLE);
}

#[test]
fn the_redraw_wait_leaves_most_of_a_short_budget_for_the_second_session() {
    assert!(redraw_wait(Duration::from_millis(40)) <= Duration::from_millis(10));
    assert_eq!(redraw_wait(Duration::ZERO), Duration::ZERO);
}
