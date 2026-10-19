use agent_desktop_core::state;

use super::*;
use crate::system::toolkit_id::ToolkitId;
use crate::system::x11::window_x11_state::WindowX11State;
use crate::tree::ancestry::{self, Ancestry};
use crate::tree::node_evidence::evidence_for;
use crate::tree::role_numbers::PUSH_BUTTON;
use crate::tree::state_bits::{ENABLED, SHOWING, VISIBLE};
use crate::tree::test_support::{read_of, words};
use crate::tree::{AtspiElement, Extents, Field};

fn rect(x: i32, y: i32, width: i32, height: i32) -> Extents {
    Extents {
        x,
        y,
        width,
        height,
    }
}

fn read_with(bits: &[u32], at: Option<Extents>) -> ObservedRead {
    let mut read = read_of(PUSH_BUTTON, "row", &[]);
    read.node.traits.states = Field::Present(words(bits));
    if let Some(at) = at {
        read.node.extents = Field::Present(at);
    }
    let mut element = AtspiElement::new(":1.1", "/p", 1);
    element.set_ancestry(ancestry::of_root(&element.clone()));
    read.node.element = element;
    read
}

fn inside_a_container(mut read: ObservedRead) -> ObservedRead {
    let frame = AtspiElement::new(":1.1", "/p", 1);
    let pane = AtspiElement::new(":1.1", "/pane", 1);
    read.node
        .element
        .set_ancestry(Ancestry::of(Some(frame), Some(pane), false));
    read
}

fn mapped(hidden: bool) -> Option<WindowX11State> {
    Some(WindowX11State {
        client_id: 7,
        hidden,
        client_origin: (302, 260),
    })
}

fn window(node: Extents) -> Placement {
    Placement {
        node: Some(node),
        frame: Some(rect(0, 0, 420, 420)),
        x11: mapped(false),
        ..Placement::default()
    }
}

fn state_in(read: &ObservedRead, placement: &Placement) -> agent_desktop_core::ElementState {
    let evidence = evidence_for(read, &read.node.element);
    live_state(read, &evidence, placement).expect("the state projects")
}

#[test]
fn a_web_node_with_visible_and_no_showing_inside_the_frame_is_not_hidden_or_offscreen() {
    let read = read_with(&[ENABLED, VISIBLE], Some(rect(542, 119, 304, 35)));
    let projected = state_in(&read, &window(rect(40, 112, 304, 35)));
    assert!(!projected.states.iter().any(|t| t == state::HIDDEN));
    assert!(!projected.states.iter().any(|t| t == state::OFFSCREEN));
    assert_eq!(projected.offscreen, Some(false));
    assert!(state::is_visible(live_bounds(&read), &projected.states));
}

#[test]
fn a_node_with_neither_bit_is_hidden_and_never_offscreen() {
    let read = read_with(&[ENABLED], Some(rect(0, 0, 10, 10)));
    let projected = state_in(&read, &window(rect(0, 0, 10, 10)));
    assert!(projected.states.iter().any(|t| t == state::HIDDEN));
    assert_eq!(projected.offscreen, None);
    assert_eq!(projected.hidden, None);
}

#[test]
fn the_gtk3_sentinel_row_is_offscreen_with_its_token_and_needs_no_placement() {
    let read = read_with(&[ENABLED, VISIBLE], Some(rect(i32::MIN, i32::MIN, 420, 22)));
    assert!(settled_by_read(&read));
    let projected = state_in(&read, &Placement::default());
    assert_eq!(projected.offscreen, Some(true));
    assert!(projected.states.iter().any(|t| t == state::OFFSCREEN));
    assert!(!state::is_visible(live_bounds(&read), &projected.states));
}

#[test]
fn a_sentinel_row_that_is_showing_is_not_the_gtk3_shape() {
    let read = read_with(
        &[ENABLED, VISIBLE, SHOWING],
        Some(rect(i32::MIN, i32::MIN, 1, 1)),
    );
    assert!(!settled_by_read(&read));
    assert_eq!(derived_offscreen(&read, &Placement::default()), None);
}

#[test]
fn a_gtk4_row_far_below_the_frame_is_offscreen_and_one_inside_is_on_screen() {
    let read = read_with(&[ENABLED, VISIBLE, SHOWING], Some(rect(0, 0, 420, 22)));
    assert!(!settled_by_read(&read));
    let below = state_in(&read, &window(rect(2, 5526, 420, 22)));
    assert_eq!(below.offscreen, Some(true));
    assert!(below.states.iter().any(|t| t == state::OFFSCREEN));
    let within = state_in(&read, &window(rect(2, 100, 420, 22)));
    assert_eq!(within.offscreen, Some(false));
    assert!(!within.states.iter().any(|t| t == state::OFFSCREEN));
}

#[test]
fn an_unread_frame_or_container_never_makes_a_node_on_screen() {
    let read = read_with(&[ENABLED, VISIBLE, SHOWING], Some(rect(0, 0, 420, 22)));
    let mut unframed = window(rect(2, 100, 420, 22));
    unframed.frame = None;
    assert_eq!(derived_offscreen(&read, &unframed), None);

    let boxed = inside_a_container(read);
    let unread = window(rect(2, 100, 420, 22));
    assert_eq!(derived_offscreen(&boxed, &unread), None);
    let mut read_container = window(rect(2, 100, 420, 22));
    read_container.container = Some(rect(0, 0, 420, 420));
    assert_eq!(derived_offscreen(&boxed, &read_container), Some(false));
    read_container.node = Some(rect(2, 5526, 420, 22));
    assert_eq!(derived_offscreen(&boxed, &read_container), Some(true));
    let mut unread_below = unread;
    unread_below.node = Some(rect(2, 5526, 420, 22));
    assert_eq!(derived_offscreen(&boxed, &unread_below), Some(true));
}

#[test]
fn an_iconified_frame_or_a_hidden_x11_window_makes_every_node_offscreen() {
    let read = read_with(&[ENABLED, VISIBLE, SHOWING], Some(rect(0, 0, 10, 10)));
    let mut iconified = window(rect(2, 100, 420, 22));
    iconified.frame_iconified = true;
    assert_eq!(derived_offscreen(&read, &iconified), Some(true));
    let mut hidden = window(rect(2, 100, 420, 22));
    hidden.x11 = mapped(true);
    assert_eq!(derived_offscreen(&read, &hidden), Some(true));
    let mut unknown = window(rect(2, 100, 420, 22));
    unknown.x11 = None;
    assert_eq!(derived_offscreen(&read, &unknown), None);
}

#[test]
fn pointer_rect_is_the_toolkits_rule_applied_to_the_three_readings() {
    let read = read_with(&[ENABLED, VISIBLE, SHOWING], Some(rect(0, 0, 498, 34)));
    let mut placed = window(rect(0, 0, 498, 34));
    placed.toolkit = ToolkitId {
        name: Some("GTK".to_owned()),
        version: Some("4.14.5".to_owned()),
    };
    let expected = pointer_bounds(
        &placed.toolkit,
        rect(0, 0, 498, 34),
        rect(0, 0, 498, 34),
        Some((302, 260)),
    )
    .map(Rect::from);
    assert_eq!(pointer_rect(&read, &placed), expected);
    placed.node = None;
    assert_eq!(pointer_rect(&read, &placed), None);
    let mut unread = read_with(&[ENABLED, VISIBLE, SHOWING], None);
    unread.node.extents = Field::Absent;
    assert_eq!(pointer_rect(&unread, &window(rect(0, 0, 1, 1))), None);
}

#[test]
fn a_global_toolkit_keeps_its_extents_and_another_falls_back_to_them_when_unprovable() {
    let read = read_with(&[ENABLED, VISIBLE, SHOWING], Some(rect(100, 137, 420, 34)));
    let evidence = evidence_for(&read, &read.node.element);
    let global =
        live_element(&read, &evidence, &Placement::default(), true).expect("global projects");
    assert_eq!(global.bounds.map(|b| (b.x, b.y)), Some((100.0, 137.0)));
    let mut unprovable = window(rect(0, 0, 420, 34));
    unprovable.node = None;
    let other = live_element(&read, &evidence, &unprovable, false).expect("projects");
    assert_eq!(other.bounds.map(|b| (b.x, b.y)), Some((100.0, 137.0)));
}
