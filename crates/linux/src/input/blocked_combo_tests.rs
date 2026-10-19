use super::*;

fn blocked(key: &str, modifiers: &[Modifier]) -> bool {
    is_blocked(&KeyCombo {
        key: key.to_string(),
        modifiers: modifiers.to_vec(),
    })
}

#[test]
fn the_listed_shortcuts_are_blocked() {
    assert!(blocked("f4", &[Modifier::Alt]));
    assert!(blocked("tab", &[Modifier::Alt]));
    assert!(blocked("l", &[Modifier::Meta]));
    assert!(blocked("delete", &[Modifier::Ctrl, Modifier::Alt]));
    assert!(blocked("backspace", &[Modifier::Ctrl, Modifier::Alt]));
    assert!(blocked("l", &[Modifier::Ctrl, Modifier::Alt]));
}

#[test]
fn a_modifier_superset_of_a_listed_shortcut_is_blocked() {
    assert!(blocked("tab", &[Modifier::Alt, Modifier::Shift]));
    assert!(blocked(
        "f4",
        &[Modifier::Shift, Modifier::Alt, Modifier::Ctrl]
    ));
}

#[test]
fn every_virtual_terminal_switch_is_blocked_and_f13_is_not() {
    for number in 1..=12 {
        let key = format!("f{number}");
        assert!(blocked(&key, &[Modifier::Ctrl, Modifier::Alt]), "{key}");
    }
    assert!(!blocked("f13", &[Modifier::Ctrl, Modifier::Alt]));
    assert!(!blocked("f3", &[Modifier::Ctrl]));
}

#[test]
fn the_super_key_alone_is_blocked_under_either_name() {
    assert!(blocked("super", &[]));
    assert!(blocked("meta", &[]));
}

#[test]
fn ordinary_shortcuts_are_not_blocked() {
    assert!(!blocked("a", &[Modifier::Ctrl]));
    assert!(!blocked("tab", &[]));
    assert!(!blocked("tab", &[Modifier::Ctrl]));
    assert!(!blocked("l", &[Modifier::Ctrl]));
    assert!(!blocked("delete", &[Modifier::Ctrl]));
    assert!(!blocked("f4", &[Modifier::Ctrl]));
}

#[test]
fn modifier_order_and_key_spelling_do_not_evade_the_list() {
    assert!(blocked("L", &[Modifier::Alt, Modifier::Ctrl]));
    assert!(blocked("TAB", &[Modifier::Alt]));
    assert!(blocked("Delete", &[Modifier::Alt, Modifier::Ctrl]));
    assert!(blocked("F4", &[Modifier::Alt, Modifier::Alt]));
}

#[test]
fn an_unknown_key_name_is_not_blocked_and_fails_in_planning_instead() {
    assert!(!blocked("hyper", &[Modifier::Alt]));
}

#[test]
fn ctrl_alt_forward_delete_is_blocked_by_its_own_name() {
    assert!(blocked("forwarddelete", &[Modifier::Ctrl, Modifier::Alt]));
    assert!(!blocked("forwarddelete", &[Modifier::Ctrl]));
}
