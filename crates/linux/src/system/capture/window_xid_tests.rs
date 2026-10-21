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
        shadow: (0, 0, 0, 0),
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
