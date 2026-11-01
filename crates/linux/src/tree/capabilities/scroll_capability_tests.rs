use std::cell::Cell;

use super::{eligible, pointer_leg};
use crate::system::display::display_cause::Cause;
use crate::system::display::display_server::DisplayServer;
use crate::system::display::display_unavailable::Unavailable;
use crate::tree::properties::node_read::COMPONENT_INTERFACE;
use crate::tree::vocabulary::role_numbers::{
    DOCUMENT_FRAME, DOCUMENT_WEB, LIST, PANEL, PUSH_BUTTON, SCROLL_PANE, VIEWPORT,
};

fn x11() -> DisplayServer {
    DisplayServer::X11(":0".to_owned())
}

fn component() -> Vec<String> {
    vec![COMPONENT_INTERFACE.to_owned()]
}

fn unavailable() -> DisplayServer {
    DisplayServer::Unavailable(Unavailable {
        cause: Cause::NotConfigured,
        detail: String::new(),
    })
}

#[test]
fn wheel_eligibility_needs_a_wheel_role_bounds_and_a_pointer_leg() {
    let component = component();
    for role in [SCROLL_PANE, VIEWPORT, LIST, DOCUMENT_FRAME, DOCUMENT_WEB] {
        assert!(eligible(role, &component, true), "role {role}");
    }
    for role in [PUSH_BUTTON, PANEL] {
        assert!(!eligible(role, &component, true), "role {role}");
    }
    assert!(!eligible(SCROLL_PANE, &[], true));
    assert!(!eligible(
        SCROLL_PANE,
        &["org.a11y.atspi.Action".to_owned()],
        true
    ));
    assert!(!eligible(SCROLL_PANE, &component, false));
}

#[test]
fn x11_has_a_pointer_leg_without_asking_for_a_hyprland_binding() {
    let asked = Cell::new(false);
    assert!(pointer_leg(&x11(), || {
        asked.set(true);
        false
    }));
    assert!(!asked.get());
}

#[test]
fn a_wayland_session_has_a_pointer_leg_only_when_bound_to_hyprland() {
    assert!(pointer_leg(&DisplayServer::Wayland, || true));
    assert!(!pointer_leg(&DisplayServer::Wayland, || false));
}

#[test]
fn an_unavailable_display_has_no_pointer_leg_even_when_bound() {
    assert!(!pointer_leg(&unavailable(), || true));
}
