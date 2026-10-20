use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use agent_desktop_core::Deadline;

use super::ControlRead;
use crate::system::display_server::{Cause, DisplayServer, Unavailable};
use crate::system::x11::control_display::route_for;

fn deadline(ms: u64) -> Deadline {
    Deadline::after(ms).expect("a deadline builds")
}

fn x11() -> DisplayServer {
    DisplayServer::X11(":99".to_owned())
}

#[test]
fn a_read_that_finishes_in_time_returns_its_value() {
    let read = route_for(x11(), None, deadline(2_000), |_| ControlRead::Ready(5_u32));
    assert_eq!(read, ControlRead::Ready(5));
}

#[test]
fn the_read_gets_the_display_of_the_session() {
    let seen = Arc::new(Mutex::new(String::new()));
    let sink = Arc::clone(&seen);
    route_for(x11(), None, deadline(2_000), move |display| {
        *sink.lock().expect("the lock is not poisoned") = display;
        ControlRead::Ready(())
    });
    assert_eq!(*seen.lock().expect("the lock is not poisoned"), ":99");
}

#[test]
fn a_read_that_never_answers_is_no_answer_within_the_deadline() {
    let (release, blocked): (Sender<()>, _) = channel();
    let started = Instant::now();
    let read = route_for(x11(), None, deadline(150), move |_| {
        let _ = blocked.recv();
        ControlRead::Ready(1_u32)
    });
    assert_eq!(read, ControlRead::Unavailable);
    assert!(started.elapsed() < Duration::from_secs(2));
    drop(release);
}

#[test]
fn a_read_that_panics_is_no_answer() {
    let read = route_for(x11(), None, deadline(2_000), |_| -> ControlRead<u32> {
        panic!("the display server sent garbage")
    });
    assert_eq!(read, ControlRead::Unavailable);
}

#[test]
fn a_wayland_session_is_not_x11_and_opens_no_connection() {
    let ran = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&ran);
    let read = route_for(DisplayServer::Wayland, None, deadline(2_000), move |_| {
        flag.store(true, Ordering::SeqCst);
        ControlRead::Ready(())
    });
    assert_eq!(read, ControlRead::NotX11);
    assert!(!ran.load(Ordering::SeqCst));
}

#[test]
fn a_session_with_no_display_is_unavailable_and_opens_no_connection() {
    let ran = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&ran);
    let gone = DisplayServer::Unavailable(Unavailable {
        cause: Cause::NotConfigured,
        detail: String::new(),
    });
    let read = route_for(gone, None, deadline(2_000), move |_| {
        flag.store(true, Ordering::SeqCst);
        ControlRead::Ready(())
    });
    assert_eq!(read, ControlRead::Unavailable);
    assert!(!ran.load(Ordering::SeqCst));
}

#[test]
fn a_wayland_session_with_an_xwayland_display_reads_it() {
    let read = route_for(
        DisplayServer::Wayland,
        Some(":7".to_owned()),
        deadline(2_000),
        ControlRead::Ready,
    );
    assert_eq!(read, ControlRead::Ready(":7".to_owned()));
}

#[test]
fn a_wayland_session_whose_xwayland_does_not_answer_is_not_x11() {
    let read = route_for(
        DisplayServer::Wayland,
        Some(":7".to_owned()),
        deadline(2_000),
        |_| -> ControlRead<u32> { ControlRead::Unavailable },
    );
    assert_eq!(read, ControlRead::NotX11);
}
