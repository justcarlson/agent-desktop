use agent_desktop_core::{LocatorField, state};

use super::resolve_states;
use crate::tree::Field;
use crate::tree::role_numbers::{
    CHECK_BOX, CHECK_MENU_ITEM, PASSWORD_TEXT, PUSH_BUTTON, PUSH_BUTTON_MENU, RADIO_BUTTON,
    RADIO_MENU_ITEM, TOGGLE_BUTTON,
};
use crate::tree::state_bits::{
    CHECKED, ENABLED, FOCUSED, INDETERMINATE, INVALID_ENTRY, PRESSED, READ_ONLY, REQUIRED,
    SENSITIVE, SHOWING,
};

const GROUP: u32 = 39;

fn words(bits: &[u32]) -> Vec<u32> {
    let mut words = vec![0u32; 2];
    for bit in bits {
        words[(*bit / 32) as usize] |= 1 << (*bit % 32);
    }
    words
}

fn tokens(role: u32, bits: &[u32]) -> Vec<String> {
    match resolve_states(&Field::Present(role), &Field::Present(words(bits))) {
        LocatorField::Known(tokens) => tokens,
        other => panic!("expected a known state list, got {other:?}"),
    }
}

fn has(role: u32, bits: &[u32], token: &str) -> bool {
    tokens(role, bits)
        .iter()
        .any(|candidate| candidate == token)
}

#[test]
fn required_is_read_from_the_second_word_only() {
    assert!(has(GROUP, &[ENABLED, REQUIRED], state::REQUIRED));
    let first_word_only = vec![1u32 << (REQUIRED % 32), 0];
    let resolved = resolve_states(&Field::Present(GROUP), &Field::Present(first_word_only));
    let LocatorField::Known(found) = resolved else {
        panic!("expected a known state list");
    };
    assert!(!found.iter().any(|token| token == state::REQUIRED));
}

#[test]
fn sensitive_without_enabled_is_not_disabled() {
    assert!(!has(PUSH_BUTTON, &[SENSITIVE, SHOWING], state::DISABLED));
    assert!(!has(PUSH_BUTTON, &[ENABLED], state::DISABLED));
    assert!(has(PUSH_BUTTON, &[SHOWING], state::DISABLED));
}

#[test]
fn a_failed_read_is_unknown_and_a_clean_read_is_an_empty_list() {
    let role = Field::Present(GROUP);
    let failed = Field::Failed("gone".to_owned());
    assert_eq!(
        resolve_states(&role, &failed as &Field<Vec<u32>>),
        LocatorField::Unknown
    );
    assert_eq!(
        resolve_states(
            &Field::Failed("gone".to_owned()),
            &Field::Present(words(&[ENABLED]))
        ),
        LocatorField::Unknown
    );
    assert_eq!(
        resolve_states(&role, &Field::Present(words(&[ENABLED]))),
        LocatorField::Known(Vec::new())
    );
}

#[test]
fn checked_needs_a_role_that_can_be_checked() {
    assert!(!has(PUSH_BUTTON, &[SENSITIVE, CHECKED], state::CHECKED));
    for role in [
        CHECK_BOX,
        RADIO_BUTTON,
        CHECK_MENU_ITEM,
        RADIO_MENU_ITEM,
        TOGGLE_BUTTON,
    ] {
        assert!(
            has(role, &[SENSITIVE, CHECKED], state::CHECKED),
            "role {role}"
        );
    }
}

#[test]
fn pressed_is_checked_on_a_toggle_and_pressed_on_a_push_button() {
    assert!(has(TOGGLE_BUTTON, &[SENSITIVE, PRESSED], state::CHECKED));
    assert!(!has(TOGGLE_BUTTON, &[SENSITIVE, PRESSED], state::PRESSED));
    for role in [PUSH_BUTTON, PUSH_BUTTON_MENU] {
        assert!(
            has(role, &[SENSITIVE, PRESSED], state::PRESSED),
            "role {role}"
        );
        assert!(
            !has(role, &[SENSITIVE, PRESSED], state::CHECKED),
            "role {role}"
        );
    }
    assert!(!has(CHECK_BOX, &[SENSITIVE, PRESSED], state::CHECKED));
}

#[test]
fn indeterminate_is_gated_like_checked() {
    assert!(has(
        CHECK_BOX,
        &[SENSITIVE, INDETERMINATE],
        state::INDETERMINATE
    ));
    assert!(!has(
        GROUP,
        &[SENSITIVE, INDETERMINATE],
        state::INDETERMINATE
    ));
}

#[test]
fn invalid_comes_from_the_entry_bit_not_the_object_bit() {
    assert!(has(GROUP, &[SENSITIVE, INVALID_ENTRY], state::INVALID));
    assert!(!has(GROUP, &[SENSITIVE, 0], state::INVALID));
    assert!(tokens(GROUP, &[SENSITIVE, 0]).is_empty());
}

#[test]
fn a_password_role_is_secure_and_an_unread_role_is_unknown() {
    assert!(has(PASSWORD_TEXT, &[SENSITIVE], state::SECURE));
    assert!(!has(GROUP, &[SENSITIVE], state::SECURE));
    assert_eq!(
        resolve_states(
            &Field::Failed("gone".to_owned()),
            &Field::Present(words(&[SENSITIVE]))
        ),
        LocatorField::Unknown
    );
}

#[test]
fn positive_bits_map_to_their_tokens() {
    assert!(has(GROUP, &[SENSITIVE, FOCUSED], state::FOCUSED));
    assert!(has(GROUP, &[SENSITIVE, READ_ONLY], state::READONLY));
}

#[test]
fn every_single_bit_on_several_roles_yields_vocabulary_tokens_only() {
    let roles = [
        GROUP,
        PUSH_BUTTON,
        CHECK_BOX,
        TOGGLE_BUTTON,
        PASSWORD_TEXT,
        PUSH_BUTTON_MENU,
    ];
    for role in roles {
        for bit in 0..=43u32 {
            let produced = tokens(role, &[bit]);
            state::assert_states_in_vocabulary(&produced);
        }
    }
}
