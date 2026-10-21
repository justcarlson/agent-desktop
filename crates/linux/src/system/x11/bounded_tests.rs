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
    let mut pending = start(move || {
        let _ = blocked.recv();
        X11State::NoWindowManager
    });
    let worker = pending.worker.take().expect("worker");
    let answer = pending.finish(deadline(150));
    assert_eq!(answer, None);
    assert!(started.elapsed() < Duration::from_secs(2));
    release.send(()).expect("late read released");
    worker.join().expect("late answer did not panic");
}

#[test]
fn a_reader_that_panics_is_unavailable_not_a_caller_failure() {
    let answer =
        start(|| -> X11State { panic!("the display server sent garbage") }).finish(deadline(2_000));
    assert_eq!(answer, None);
}

#[test]
fn work_done_before_the_wait_overlaps_the_read() {
    let (done, ready) = channel();
    let pending = start(move || {
        let answer = Fixed(X11State::NoWindowManager).read();
        done.send(()).expect("ready");
        answer
    });
    ready
        .recv_timeout(Duration::from_secs(2))
        .expect("read finished");
    let started = Instant::now();
    let answer = pending.finish(deadline(2_000));
    assert_eq!(answer, Some(X11State::NoWindowManager));
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[test]
fn a_silent_handshake_releases_the_socket_before_returning() {
    use std::io::Read;
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
    let port = listener.local_addr().expect("address").port();
    let display = format!("127.0.0.1:{}", port - 6000);
    let (closed, observed) = channel();
    let server = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().expect("accepted");
        socket
            .set_read_timeout(Some(Duration::from_secs(3)))
            .expect("timeout");
        let mut bytes = [0; 1024];
        loop {
            match socket.read(&mut bytes) {
                Ok(0) => break,
                Ok(_) => {}
                Err(error) => panic!("client did not close: {error}"),
            }
        }
        closed.send(()).expect("observed close");
    });
    let (ended, retired) = channel();
    let pending = start(move || {
        let answer = super::connect(&display);
        let _ = ended.send(());
        answer.is_ok()
    });
    assert_eq!(pending.finish(deadline(150)), None);
    observed
        .recv_timeout(Duration::from_secs(2))
        .expect("socket released at deadline");
    retired
        .recv_timeout(Duration::from_secs(2))
        .expect("worker retired");
    server.join().expect("server ended");
}
