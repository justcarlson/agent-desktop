use agent_desktop_core::{ErrorCode, LocatorEvidence, LocatorField, Rect, state};

use super::*;
use crate::tree::node_evidence::evidence_for;
use crate::tree::role_numbers::{PASSWORD_TEXT, PUSH_BUTTON, SLIDER};
use crate::tree::state_bits::{ENABLED, SENSITIVE, SHOWING, VISIBLE};
use crate::tree::test_support::{read_of, words};
use crate::tree::{AtspiElement, Extents, Field, ancestry};

fn framed() -> AtspiElement {
    let mut element = AtspiElement::new(":1.1", "/p", 1);
    element.set_ancestry(ancestry::of_root(&element.clone()));
    element
}

fn evidence_of(read: &ObservedRead) -> LocatorEvidence {
    evidence_for(read, &framed())
}

fn with_states(mut read: ObservedRead, bits: &[u32]) -> ObservedRead {
    read.node.traits.states = Field::Present(words(bits));
    read
}

fn with_extents(mut read: ObservedRead, x: i32, y: i32, width: i32, height: i32) -> ObservedRead {
    read.node.extents = Field::Present(Extents {
        x,
        y,
        width,
        height,
    });
    read
}

fn state_of(read: &ObservedRead) -> agent_desktop_core::ElementState {
    state_in(read, &Placement::default())
}

fn state_in(read: &ObservedRead, placement: &Placement) -> agent_desktop_core::ElementState {
    live_state(read, &evidence_of(read), placement).expect("the state projects")
}

fn hidden(read: &ObservedRead) -> bool {
    state_of(read)
        .states
        .iter()
        .any(|token| token == state::HIDDEN)
}

#[test]
fn a_node_with_neither_visible_nor_showing_is_hidden_and_not_visible_to_core() {
    let read = with_states(read_of(PUSH_BUTTON, "b", &[]), &[ENABLED]);
    let projected = state_of(&read);
    assert!(projected.states.iter().any(|token| token == state::HIDDEN));
    assert!(!state::is_visible(live_bounds(&read), &projected.states));
}

#[test]
fn visible_without_showing_and_showing_alone_are_not_hidden() {
    let visible = with_states(read_of(PUSH_BUTTON, "b", &[]), &[ENABLED, VISIBLE]);
    assert!(!hidden(&visible));
    let showing_only = with_states(read_of(PUSH_BUTTON, "b", &[]), &[ENABLED, SHOWING]);
    assert!(!hidden(&showing_only));
    let showing = with_states(read_of(PUSH_BUTTON, "b", &[]), &[ENABLED, VISIBLE, SHOWING]);
    assert!(!hidden(&showing));
    let projected = state_of(&showing);
    assert!(state::is_visible(live_bounds(&showing), &projected.states));
}

#[test]
fn a_failed_state_read_is_the_incomplete_answer_and_never_an_empty_state_list() {
    let mut read = read_of(PUSH_BUTTON, "b", &[]);
    read.node.traits.states = Field::Failed("no answer".to_owned());
    let error =
        live_state(&read, &evidence_of(&read), &Placement::default()).expect_err("incomplete");
    assert_eq!(error.code, ErrorCode::AppUnresponsive);
    assert!(error.is_explicitly_retryable());
}

#[test]
fn off_screen_gtk3_content_is_neither_hidden_nor_bounded() {
    let read = with_extents(
        with_states(read_of(PUSH_BUTTON, "tab", &[]), &[ENABLED, VISIBLE]),
        i32::MIN,
        i32::MIN,
        420,
        22,
    );
    assert!(!hidden(&read));
    assert_eq!(live_bounds(&read), None);
}

#[test]
fn enabled_comes_from_either_bit_and_disabled_needs_a_successful_read_with_neither() {
    for bits in [&[ENABLED][..], &[SENSITIVE][..]] {
        let read = with_states(read_of(PUSH_BUTTON, "b", &[]), bits);
        let projected = state_of(&read);
        assert_eq!(projected.enabled, Some(true));
        assert!(
            !projected
                .states
                .iter()
                .any(|token| token == state::DISABLED)
        );
        assert_eq!(projected.offscreen, None);
        assert_eq!(projected.hidden, None);
    }
    let neither = with_states(read_of(PUSH_BUTTON, "b", &[]), &[VISIBLE]);
    let projected = state_of(&neither);
    assert_eq!(projected.enabled, Some(false));
    assert!(
        projected
            .states
            .iter()
            .any(|token| token == state::DISABLED)
    );
}

#[test]
fn zero_and_negative_extents_are_values_and_the_sentinel_or_a_missing_read_is_none() {
    let zero = with_extents(read_of(PUSH_BUTTON, "z", &[]), 0, 0, 710, 0);
    assert_eq!(
        live_bounds(&zero),
        Some(Rect {
            x: 0.0,
            y: 0.0,
            width: 710.0,
            height: 0.0
        })
    );
    let negative = with_extents(read_of(PUSH_BUTTON, "n", &[]), -1, -1, -1, -1);
    assert_eq!(
        live_bounds(&negative).map(|rect| (rect.width, rect.height)),
        Some((-1.0, -1.0))
    );
    let sentinel = with_extents(read_of(PUSH_BUTTON, "s", &[]), i32::MIN, i32::MIN, 1, 1);
    assert_eq!(live_bounds(&sentinel), None);
    let mut absent = read_of(PUSH_BUTTON, "a", &[]);
    absent.node.extents = Field::Absent;
    assert_eq!(live_bounds(&absent), None);
    absent.node.extents = Field::Failed("x".to_owned());
    assert_eq!(live_bounds(&absent), None);
}

#[test]
fn a_value_is_read_as_the_node_gives_it_and_a_failed_read_is_incomplete() {
    let mut slider = read_of(SLIDER, "s", &[]);
    slider.value.value = Field::Present("7".to_owned());
    assert_eq!(
        live_value(&evidence_of(&slider)).expect("reads"),
        Some("7".to_owned())
    );
    let plain = read_of(PUSH_BUTTON, "b", &[]);
    assert_eq!(live_value(&evidence_of(&plain)).expect("reads"), None);
    let mut failed = read_of(SLIDER, "s", &[]);
    failed.value.value = Field::Failed("x".to_owned());
    let error = live_value(&evidence_of(&failed)).expect_err("incomplete");
    assert_eq!(error.code, ErrorCode::AppUnresponsive);
}

#[test]
fn a_secure_fields_value_is_withheld_even_when_one_was_read() {
    let mut password = read_of(PASSWORD_TEXT, "p", &[]);
    password.value.value = Field::Present("hunter2".to_owned());
    let evidence = evidence_of(&password);
    assert!(
        evidence
            .states
            .known()
            .is_some_and(|tokens| tokens.iter().any(|token| token == state::SECURE))
    );
    assert_eq!(live_value(&evidence).expect("reads"), None);
    assert_eq!(state_of(&password).value, None);
}

#[test]
fn actions_map_by_the_capability_table_and_a_failed_list_is_incomplete() {
    let button = read_of(PUSH_BUTTON, "b", &["click"]);
    assert_eq!(
        live_actions(&evidence_of(&button)).expect("reads"),
        vec![agent_desktop_core::capability::CLICK.to_owned()]
    );
    let none = read_of(PUSH_BUTTON, "b", &[]);
    assert!(live_actions(&evidence_of(&none)).expect("reads").is_empty());
    let mut failed = read_of(PUSH_BUTTON, "b", &["click"]);
    failed.node.actions = Field::Failed("x".to_owned());
    assert_eq!(
        live_actions(&evidence_of(&failed))
            .expect_err("incomplete")
            .code,
        ErrorCode::AppUnresponsive
    );
}

#[test]
fn the_live_element_is_identity_state_bounds_and_actions_of_one_read() {
    let read = with_states(
        with_extents(read_of(PUSH_BUTTON, "Save", &["click"]), 3, 4, 50, 20),
        &[ENABLED, VISIBLE, SHOWING],
    );
    let evidence = evidence_of(&read);
    let element = live_element(&read, &evidence, &Placement::default(), true).expect("projects");
    assert_eq!(
        element.identity.name,
        LocatorField::Known("Save".to_owned())
    );
    assert_eq!(element.state.role, "button");
    assert_eq!(element.bounds.map(|rect| rect.width), Some(50.0));
    assert_eq!(
        element.available_actions,
        vec![agent_desktop_core::capability::CLICK.to_owned()]
    );
    assert!(element.states_complete);
}
