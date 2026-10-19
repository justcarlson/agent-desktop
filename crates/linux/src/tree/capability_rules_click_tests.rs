use super::*;
use crate::tree::role_numbers::{CHECK_BOX, COMBO_BOX};

#[test]
fn a_switch_with_only_a_toggle_verb_also_takes_a_click() {
    use crate::tree::role_numbers::TOGGLE_BUTTON;
    for role in [CHECK_BOX, TOGGLE_BUTTON] {
        for verb in ["toggle", "switch", "uncheck"] {
            let spec = Spec {
                names: &[verb],
                ..plain(role)
            };
            assert!(tokens_of(&spec).contains(&"Click"), "{role} {verb}");
        }
    }
}

#[test]
fn a_toggle_verb_on_a_button_or_a_plain_check_verb_adds_no_click_rule() {
    let button = Spec {
        names: &["toggle"],
        ..plain(PUSH_BUTTON)
    };
    assert!(!tokens_of(&button).contains(&"Click"));
}

fn tokens_with_states(role: u32, names: &[&str], bits: &[u32]) -> Vec<&'static str> {
    let mut raw = node(&Spec {
        names,
        ..plain(role)
    });
    raw.traits.states = Field::Present(words(bits));
    rule_tokens(&Facts::of(&raw).expect("every read present"))
}

#[test]
fn a_disabled_button_keeps_the_click_a_live_one_would_carry() {
    for role in [PUSH_BUTTON, CHECK_BOX] {
        let disabled = tokens_with_states(role, &["doDefault"], &[]);
        assert!(disabled.contains(&"Click"), "{role}");
        assert!(!tokens_with_states(role, &["doDefault"], &[SENSITIVE]).contains(&"Click"));
    }
    assert!(!tokens_with_states(PANEL, &["doDefault"], &[]).contains(&"Click"));
}

#[test]
fn a_read_only_node_is_never_given_the_disabled_click() {
    assert!(!tokens_with_states(PUSH_BUTTON, &[], &[READ_ONLY]).contains(&"Click"));
}

#[test]
fn a_popup_dropdown_combo_with_no_selection_interface_gets_select() {
    let closed = Spec {
        bits: &[HAS_POPUP],
        children: 1,
        ..plain(COMBO_BOX)
    };
    assert!(tokens_of(&closed).contains(&"Select"));
    let bare = Spec {
        children: 1,
        ..plain(COMBO_BOX)
    };
    assert!(!tokens_of(&bare).contains(&"Select"));
    let many = Spec {
        bits: &[HAS_POPUP],
        children: 3,
        ..plain(COMBO_BOX)
    };
    assert!(!tokens_of(&many).contains(&"Select"));
}
