use std::sync::mpsc::{Sender, channel};
use std::time::{Duration, Instant};

use agent_desktop_core::Deadline;

use super::start;
use crate::system::x11::{X11Source, X11State};

struct Fixed(X11State);

impl X11Source for Fixed {
    fn read(&self) -> X11State {
        self.0.clone()
    }
}

fn deadline(ms: u64) -> Deadline {
    Deadline::after(ms).expect("deadline")
}

#[test]
fn a_prompt_source_answers_through_the_wait() {
    let answer = start(|| X11State::NoWindowManager)
        .finish(deadline(2_000))
        .expect("answered");
    assert_eq!(answer, X11State::NoWindowManager);
}

#[test]
fn a_reader_that_never_answers_is_unavailable_within_the_deadline() {
    let (release, blocked): (Sender<()>, _) = channel();
    let started = Instant::now();
    let answer = start(move || {
        let _ = blocked.recv();
        X11State::NoWindowManager
    })
    .finish(deadline(150));
    assert_eq!(answer, None);
    assert!(started.elapsed() < Duration::from_secs(2));
    drop(release);
}

#[test]
fn a_reader_that_panics_is_unavailable_not_a_caller_failure() {
    let answer =
        start(|| -> X11State { panic!("the display server sent garbage") }).finish(deadline(2_000));
    assert_eq!(answer, None);
}

#[test]
fn work_done_before_the_wait_overlaps_the_read() {
    let pending = start(move || Fixed(X11State::NoWindowManager).read());
    std::thread::sleep(Duration::from_millis(50));
    let started = Instant::now();
    let answer = pending.finish(deadline(2_000));
    assert_eq!(answer, Some(X11State::NoWindowManager));
    assert!(started.elapsed() < Duration::from_secs(2));
}
