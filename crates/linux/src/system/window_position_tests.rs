use super::*;
use crate::system::process_facts::ProcessFacts;
use crate::system::registry_app::assemble;
use crate::system::toolkit_id::ToolkitId;
use crate::system::window_listing::collect;
use crate::tree::test_support::{read_of, words};
use crate::tree::{AtspiElement, Field};

fn windows(toolkit: (&str, &str)) -> Vec<ListedWindow> {
    let app = assemble(
        AtspiElement::new(":1.9", "/org/a11y/atspi/accessible/root", 321),
        ProcessFacts {
            comm: "x".to_owned(),
            exe_basename: Some("x".to_owned()),
            instance: "boot:42".to_owned(),
            web_family: None,
        },
        Some(Some("App".to_owned())),
        ToolkitId {
            name: Some(toolkit.0.to_owned()),
            version: Some(toolkit.1.to_owned()),
        },
    );
    let mut node = read_of(23, "Doc", &[]).node;
    node.element = AtspiElement::new(":1.9", "/org/a11y/atspi/accessible/1", 321);
    node.traits.interfaces = Field::Present(vec![crate::tree::COMPONENT_INTERFACE.to_owned()]);
    node.traits.states = Field::Present(words(&[25, 1]));
    collect(&[(0, &app)], &[vec![node]])
}

fn client(id: u32, x: i32, y: i32) -> X11Client {
    X11Client {
        id,
        pid: Some(321),
        title: "Doc".to_owned(),
        extents: None,
        bounds: Some(Extents {
            x,
            y,
            width: 10,
            height: 10,
        }),
    }
}

fn ready(clients: Vec<X11Client>) -> X11State {
    X11State::Ready {
        active: None,
        clients,
    }
}

#[test]
fn a_window_without_global_position_takes_the_origin_of_its_unique_client() {
    let mut listed = windows(("gtk", "4.14.5"));
    assert_eq!(listed[0].info.bounds, None);
    place_windows(&mut listed, &ready(vec![client(7, 460, 165)]));
    let bounds = listed[0].info.bounds.expect("a position");
    assert_eq!((bounds.x, bounds.y), (460.0, 165.0));
    assert_eq!((bounds.width, bounds.height), (10.0, 10.0));
}

#[test]
fn a_window_that_maps_to_two_clients_has_no_position() {
    let mut listed = windows(("gtk", "4.14.5"));
    place_windows(
        &mut listed,
        &ready(vec![client(7, 460, 165), client(8, 20, 20)]),
    );
    assert_eq!(listed[0].info.bounds, None);
}

#[test]
fn a_window_has_no_position_when_the_display_server_did_not_answer() {
    for state in [X11State::Unavailable, X11State::NoWindowManager] {
        let mut listed = windows(("gtk", "4.14.5"));
        place_windows(&mut listed, &state);
        assert_eq!(listed[0].info.bounds, None);
    }
}

#[test]
fn an_unreadable_toolkit_is_treated_as_having_no_global_position() {
    let mut listed = windows(("clutter", "1.0"));
    place_windows(&mut listed, &ready(vec![client(7, 300, 90)]));
    assert_eq!(listed[0].info.bounds.map(|rect| rect.x), Some(300.0));
}

#[test]
fn a_global_position_toolkit_keeps_the_extents_it_reported() {
    let mut listed = windows(("gtk", "3.24.41"));
    let reported = listed[0].info.bounds;
    assert!(reported.is_some());
    place_windows(&mut listed, &ready(vec![client(7, 460, 165)]));
    assert_eq!(listed[0].info.bounds, reported);
}
