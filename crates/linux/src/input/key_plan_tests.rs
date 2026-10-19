use super::*;

fn combo(key: &str, modifiers: &[Modifier]) -> KeyCombo {
    KeyCombo {
        key: key.to_string(),
        modifiers: modifiers.to_vec(),
    }
}

fn key(code: u8, down: bool) -> Step {
    Step::Key { code, down }
}

#[test]
fn ctrl_a_presses_ctrl_then_a_then_releases_in_reverse() {
    let plan = plan_combo(&Keymap::sample(), &combo("a", &[Modifier::Ctrl])).unwrap();
    assert_eq!(
        plan.steps,
        vec![key(9, true), key(10, true), key(10, false), key(9, false)]
    );
    assert_eq!(plan.release_origin, None);
}

#[test]
fn modifiers_release_in_the_reverse_of_the_order_given() {
    let plan = plan_combo(
        &Keymap::sample(),
        &combo("a", &[Modifier::Ctrl, Modifier::Shift]),
    )
    .unwrap();
    assert_eq!(
        plan.steps,
        vec![
            key(9, true),
            key(8, true),
            key(10, true),
            key(10, false),
            key(8, false),
            key(9, false),
        ]
    );
    let swapped = plan_combo(
        &Keymap::sample(),
        &combo("a", &[Modifier::Shift, Modifier::Ctrl]),
    )
    .unwrap();
    assert_eq!(swapped.steps[0], key(8, true));
    assert_eq!(swapped.steps[5], key(8, false));
}

#[test]
fn an_uppercase_key_gets_a_shift_wrapper_from_the_mapping_level() {
    let plan = plan_combo(&Keymap::sample(), &combo("A", &[])).unwrap();
    assert_eq!(
        plan.steps,
        vec![key(8, true), key(10, true), key(10, false), key(8, false)]
    );
}

#[test]
fn an_uppercase_key_with_explicit_shift_does_not_press_shift_twice() {
    let plan = plan_combo(&Keymap::sample(), &combo("A", &[Modifier::Shift])).unwrap();
    let shift_downs = plan
        .steps
        .iter()
        .filter(|step| **step == key(8, true))
        .count();
    assert_eq!(shift_downs, 1);
}

#[test]
fn a_key_missing_from_the_layout_is_refused_before_any_event() {
    let error = plan_combo(&Keymap::sample(), &combo("你", &[])).unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidArgs);
}

#[test]
fn a_modifier_the_keyboard_lacks_is_refused() {
    let bare = Keymap::parse(8, 2, vec![0x61, 0x41], 0, Vec::new());
    let error = plan_combo(&bare, &combo("a", &[Modifier::Ctrl])).unwrap_err();
    assert_eq!(error.code, ErrorCode::ActionNotSupported);
}

#[test]
fn a_repeated_modifier_is_pressed_once() {
    let plan = plan_combo(
        &Keymap::sample(),
        &combo("a", &[Modifier::Ctrl, Modifier::Ctrl]),
    )
    .unwrap();
    assert_eq!(plan.steps.len(), 4);
}

#[test]
fn an_unknown_key_name_is_invalid_args() {
    let error = plan_combo(&Keymap::sample(), &combo("hyper", &[])).unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidArgs);
}

#[test]
fn keysym_steps_wrap_the_key_in_its_modifiers_in_reverse_release_order() {
    let steps = plan_keysyms(&combo("a", &[Modifier::Ctrl, Modifier::Alt])).unwrap();
    let edge = |keysym, down| KeysymStep { keysym, down };
    assert_eq!(
        steps,
        vec![
            edge(0xffe3, true),
            edge(0xffe9, true),
            edge(0x61, true),
            edge(0x61, false),
            edge(0xffe9, false),
            edge(0xffe3, false),
        ]
    );
}

#[test]
fn keysym_steps_keep_an_uppercase_keysym_without_a_shift_wrapper() {
    let steps = plan_keysyms(&combo("A", &[])).unwrap();
    assert_eq!(steps.len(), 2);
    assert_eq!(steps[0].keysym, 0x41);
}
