use super::*;

#[test]
fn single_characters_keep_their_case() {
    assert_eq!(keysym_for_key("a").unwrap(), 0x61);
    assert_eq!(keysym_for_key("A").unwrap(), 0x41);
    assert_eq!(keysym_for_key("7").unwrap(), 0x37);
}

#[test]
fn named_keys_resolve_without_regard_to_case() {
    assert_eq!(keysym_for_key("Return").unwrap(), 0xff0d);
    assert_eq!(keysym_for_key("enter").unwrap(), 0xff0d);
    assert_eq!(keysym_for_key("PageDown").unwrap(), 0xff56);
    assert_eq!(keysym_for_key("space").unwrap(), 0x20);
}

#[test]
fn function_keys_run_from_f1_to_f24() {
    assert_eq!(keysym_for_key("f1").unwrap(), 0xffbe);
    assert_eq!(keysym_for_key("f12").unwrap(), 0xffc9);
    assert_eq!(keysym_for_key("f13").unwrap(), 0xffca);
    assert_eq!(keysym_for_key("F24").unwrap(), 0xffd5);
}

#[test]
fn f25_f0_and_padded_function_keys_are_refused() {
    for name in ["f25", "f0", "f01", "f1x"] {
        let error = keysym_for_key(name).unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidArgs, "{name}");
    }
}

#[test]
fn an_unknown_name_is_invalid_args_and_lists_the_accepted_forms() {
    let error = keysym_for_key("hyper").unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidArgs);
    let suggestion = error.suggestion.unwrap();
    assert!(suggestion.contains("f1-f24") && suggestion.contains("pagedown"));
}

#[test]
fn cmd_is_not_a_linux_key_name() {
    assert_eq!(
        keysym_for_key("cmd").unwrap_err().code,
        ErrorCode::InvalidArgs
    );
}

#[test]
fn modifier_names_alias_to_one_keysym() {
    assert_eq!(
        keysym_for_key("control").unwrap(),
        keysym_for_key("ctrl").unwrap()
    );
    assert_eq!(
        keysym_for_key("meta").unwrap(),
        keysym_for_key("super").unwrap()
    );
}

#[test]
fn characters_outside_latin_one_use_the_unicode_keysym_form() {
    assert_eq!(keysym_for_char('é'), 0xe9);
    assert_eq!(keysym_for_char('你'), 0x0100_0000 + 0x4f60);
    assert_eq!(keysym_for_char('\n'), KEYSYM_RETURN);
    assert_eq!(keysym_for_char('\t'), KEYSYM_TAB);
}

#[test]
fn every_modifier_has_its_own_keysym() {
    let all = [
        Modifier::Shift,
        Modifier::Ctrl,
        Modifier::Alt,
        Modifier::Meta,
    ];
    let mut keysyms: Vec<u32> = all.iter().map(modifier_keysym).collect();
    keysyms.sort_unstable();
    keysyms.dedup();
    assert_eq!(keysyms.len(), 4);
}
