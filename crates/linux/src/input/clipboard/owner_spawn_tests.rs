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
}

impl HandshakeProbe for Script {
    fn outcome(&mut self) -> Option<Outcome> {
        self.outcomes.pop_front().flatten()
    }

    fn exit_reason(&mut self) -> Option<String> {
        self.exit.clone()
    }
}

fn deadline(ms: u64) -> Deadline {
    Deadline::after(ms).expect("a deadline")
}

#[test]
fn a_record_written_on_the_way_out_still_counts_when_the_child_has_exited() {
    let mut probe = Script {
        outcomes: VecDeque::from([None, Some(Outcome::Failed("late".into()))]),
        exit: Some("status 1".into()),
    };
    assert_eq!(
        await_handshake(&mut probe, deadline(2_000)),
        Handshake::Failed("late".into())
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
