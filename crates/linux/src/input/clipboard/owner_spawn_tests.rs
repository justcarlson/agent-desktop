use std::collections::VecDeque;
use std::fs;
use std::io::Read;
use std::os::fd::AsRawFd;

use agent_desktop_core::Deadline;
use rustix::io::fcntl_getfd;

use super::*;
use crate::input::clipboard::payload::{Target, unframe};
use crate::input::clipboard::test_nonce::test_nonce;

struct Script {
    outcomes: VecDeque<Option<Outcome>>,
    exit: Option<String>,
    polls: usize,
}

impl HandshakeProbe for Script {
    fn outcome(&mut self) -> Option<Outcome> {
        self.polls += 1;
        self.outcomes.pop_front().flatten()
    }

    fn exit_reason(&mut self) -> Option<String> {
        self.exit.clone()
    }
}

fn record() -> OwnerRecord {
    OwnerRecord {
        pid: 5,
        start_ticks: 6,
        display: ":7".into(),
        window: 8,
        nonce: "abcdef01".into(),
    }
}

fn deadline(ms: u64) -> Deadline {
    Deadline::after(ms).expect("a deadline")
}

#[test]
fn a_record_that_appears_after_a_few_polls_is_the_handshake() {
    let mut probe = Script {
        outcomes: VecDeque::from([None, None, Some(Outcome::Ready(record()))]),
        exit: None,
        polls: 0,
    };
    assert_eq!(
        await_handshake(&mut probe, deadline(2_000)),
        Handshake::Ready(record())
    );
    assert_eq!(probe.polls, 3);
}

#[test]
fn an_error_record_is_a_failed_handshake() {
    let mut probe = Script {
        outcomes: VecDeque::from([Some(Outcome::Failed("no payload".into()))]),
        exit: None,
        polls: 0,
    };
    assert_eq!(
        await_handshake(&mut probe, deadline(2_000)),
        Handshake::Failed("no payload".into())
    );
}

#[test]
fn a_child_that_exited_is_reported_at_once_and_not_after_the_deadline() {
    let mut probe = Script {
        outcomes: VecDeque::new(),
        exit: Some("status 3".into()),
        polls: 0,
    };
    let started = std::time::Instant::now();
    assert_eq!(
        await_handshake(&mut probe, deadline(5_000)),
        Handshake::Exited("status 3".into())
    );
    assert!(started.elapsed() < Duration::from_secs(1));
}

#[test]
fn a_record_written_on_the_way_out_still_counts_when_the_child_has_exited() {
    let mut probe = Script {
        outcomes: VecDeque::from([None, Some(Outcome::Failed("late".into()))]),
        exit: Some("status 1".into()),
        polls: 0,
    };
    assert_eq!(
        await_handshake(&mut probe, deadline(2_000)),
        Handshake::Failed("late".into())
    );
}

#[test]
fn no_record_before_the_deadline_is_a_timeout() {
    let mut probe = Script {
        outcomes: VecDeque::new(),
        exit: None,
        polls: 0,
    };
    assert_eq!(
        await_handshake(&mut probe, deadline(60)),
        Handshake::TimedOut
    );
}

fn sample() -> Payload {
    Payload {
        targets: vec![Target {
            name: "UTF8_STRING".into(),
            bytes: b"secret text".to_vec(),
        }],
    }
}

#[test]
fn the_payload_file_is_anonymous_inheritable_and_holds_the_framed_payload() {
    let nonce = test_nonce();
    let descriptor = payload_file(&sample(), &nonce).expect("a payload file");
    let link = fs::read_link(format!("/proc/self/fd/{}", descriptor.as_raw_fd()))
        .expect("the descriptor names a file");
    assert!(
        link.to_string_lossy().starts_with("/memfd:ad-payload-"),
        "{link:?}"
    );
    assert!(
        !fcntl_getfd(&descriptor)
            .expect("flags")
            .contains(FdFlags::CLOEXEC)
    );
    let mut reopened =
        File::open(format!("/proc/self/fd/{}", descriptor.as_raw_fd())).expect("reopen");
    let mut bytes = Vec::new();
    reopened.read_to_end(&mut bytes).expect("read");
    assert_eq!(unframe(&bytes).expect("a payload"), sample());
}

#[test]
fn a_payload_never_exists_as_a_file_on_disk() {
    let nonce = test_nonce();
    let descriptor = payload_file(&sample(), &nonce).expect("a payload file");
    let target = fs::read_link(format!("/proc/self/fd/{}", descriptor.as_raw_fd()))
        .expect("a link")
        .to_string_lossy()
        .into_owned();
    assert!(
        !target.starts_with("/tmp") && !target.starts_with("/home"),
        "{target}"
    );
    drop(descriptor);
}

#[test]
fn nonces_are_valid_record_nonces_and_differ() {
    let first = new_nonce();
    let second = new_nonce();
    assert!(owner_record::valid_nonce(&first), "{first}");
    assert_ne!(first, second);
}

#[test]
fn a_finished_owner_is_reaped_and_leaves_no_zombie() {
    let child = std::process::Command::new("true").spawn().expect("a child");
    let pid = child.id();
    reap_when_done(child);
    let gone = (0..200).any(|_| {
        sleep(Duration::from_millis(25));
        !Path::new(&format!("/proc/{pid}")).exists()
    });
    assert!(gone, "the finished child must be collected");
}
