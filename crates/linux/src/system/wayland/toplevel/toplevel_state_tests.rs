use std::os::unix::net::UnixStream;

use wayland_client::{Connection, Proxy};

use super::*;

fn state_with(ext: &[bool], wlr: &[bool]) -> (ToplevelState, UnixStream) {
    let (ours, theirs) = UnixStream::pair().expect("pair");
    let backend = Connection::from_socket(ours)
        .expect("connection")
        .backend()
        .downgrade();
    let state = ToplevelState {
        ext: ext
            .iter()
            .map(|answered| {
                let record = ExtRecord {
                    answered: *answered,
                    ..ExtRecord::default()
                };
                (ExtForeignToplevelHandleV1::inert(backend.clone()), record)
            })
            .collect(),
        wlr: wlr
            .iter()
            .map(|answered| {
                let record = WlrRecord {
                    answered: *answered,
                    ..WlrRecord::default()
                };
                (ZwlrForeignToplevelHandleV1::inert(backend.clone()), record)
            })
            .collect(),
        ..ToplevelState::default()
    };
    (state, theirs)
}

#[test]
fn a_handle_announced_after_the_mapping_requests_does_not_hold_the_settle_open() {
    let (state, _peer) = state_with(&[true, false], &[true, false]);

    assert!(state.mapped((1, 1)));
}

#[test]
fn a_requested_handle_without_an_answer_keeps_the_settle_waiting() {
    let (state, _peer) = state_with(&[true, false], &[true]);

    assert!(!state.mapped((2, 1)));
}
