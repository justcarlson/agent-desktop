use super::*;
use crate::system::x11::X11Client;
use crate::tree::Extents;

fn frame(pid: u32, title: &str) -> FrameFacts {
    FrameFacts {
        pid,
        title: title.to_owned(),
        extents: None,
        global_position: false,
    }
}

fn client(id: u32, pid: u32, title: &str) -> X11Client {
    X11Client {
        id,
        pid: Some(pid),
        title: title.to_owned(),
        extents: None,
        bounds: Some(Extents {
            x: 0,
            y: 0,
            width: 10,
            height: 10,
        }),
        hidden: false,
    }
}

fn ready(clients: Vec<X11Client>) -> X11State {
    X11State::Ready {
        active: None,
        clients,
    }
}

#[test]
fn one_matching_client_is_its_id() {
    let state = ready(vec![client(7, 1, "A"), client(8, 1, "B")]);
    assert_eq!(outcome_of(state, &frame(1, "B")), XidOutcome::One(8));
}

#[test]
fn two_matching_clients_are_several() {
    let state = ready(vec![client(7, 1, "A"), client(8, 1, "A")]);
    assert_eq!(
        outcome_of(state, &frame(1, "A")),
        XidOutcome::Several(vec![7, 8])
    );
}

#[test]
fn no_matching_client_with_a_manager_is_no_client() {
    let state = ready(vec![client(7, 2, "A")]);
    assert_eq!(outcome_of(state, &frame(1, "A")), XidOutcome::NoClient);
}

#[test]
fn the_server_states_without_a_client_list_keep_their_cause() {
    assert_eq!(
        outcome_of(X11State::NoWindowManager, &frame(1, "A")),
        XidOutcome::NoManager
    );
    assert_eq!(
        outcome_of(X11State::Unavailable, &frame(1, "A")),
        XidOutcome::Unavailable
    );
}

#[test]
fn only_one_is_an_id_and_every_other_outcome_has_its_code() {
    assert_eq!(xid_or_error(XidOutcome::One(5)).expect("an id"), 5);
    let ambiguous = xid_or_error(XidOutcome::Several(vec![1, 2, 3])).expect_err("ambiguous");
    assert_eq!(ambiguous.code, ErrorCode::AmbiguousTarget);
    assert_eq!(
        ambiguous
            .details
            .as_ref()
            .and_then(|d| d["candidates"].as_u64()),
        Some(3)
    );
    assert_eq!(
        xid_or_error(XidOutcome::NoClient).expect_err("none").code,
        ErrorCode::ActionFailed
    );
    assert_eq!(
        xid_or_error(XidOutcome::NoManager)
            .expect_err("no manager")
            .code,
        ErrorCode::PlatformNotSupported
    );
    assert_eq!(
        xid_or_error(XidOutcome::Unavailable)
            .expect_err("no display")
            .code,
        ErrorCode::PlatformNotSupported
    );
}
