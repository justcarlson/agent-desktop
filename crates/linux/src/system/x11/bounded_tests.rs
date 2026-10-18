use std::sync::mpsc::{Sender, channel};
use std::time::{Duration, Instant};

use agent_desktop_core::Deadline;

use super::*;

struct Fixed(X11State);

impl X11Source for Fixed {
    fn read(&self) -> X11State {
        self.0.clone()
    }
}

struct Blocked(std::sync::mpsc::Receiver<()>);

impl X11Source for Blocked {
    fn read(&self) -> X11State {
        let _ = self.0.recv();
        X11State::NoWindowManager
    }
}

struct Panics;

impl X11Source for Panics {
    fn read(&self) -> X11State {
        panic!("the display server sent garbage")
    }
}

fn deadline(ms: u64) -> Deadline {
    Deadline::after(ms).expect("deadline")
}

#[test]
fn a_prompt_source_answers_through_the_wait() {
    let answer = start(Fixed(X11State::NoWindowManager)).finish(deadline(2_000));
    assert_eq!(answer, X11State::NoWindowManager);
}

#[test]
fn a_source_that_never_answers_is_unavailable_within_the_deadline() {
    let (release, blocked): (Sender<()>, _) = channel();
    let started = Instant::now();
    let answer = start(Blocked(blocked)).finish(deadline(150));
    assert_eq!(answer, X11State::Unavailable);
    assert!(started.elapsed() < Duration::from_secs(2));
    drop(release);
}

#[test]
fn a_source_that_panics_is_unavailable_not_a_caller_failure() {
    let answer = start(Panics).finish(deadline(2_000));
    assert_eq!(answer, X11State::Unavailable);
}

#[test]
fn work_done_before_the_wait_overlaps_the_read() {
    let pending = start(Fixed(X11State::NoWindowManager));
    std::thread::sleep(Duration::from_millis(50));
    let started = Instant::now();
    let answer = pending.finish(deadline(2_000));
    assert_eq!(answer, X11State::NoWindowManager);
    assert!(started.elapsed() < Duration::from_millis(40));
}
