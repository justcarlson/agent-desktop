use super::*;

#[test]
fn a_bool_reply_is_read_as_the_flag() {
    let on = br#"{"option": "ecosystem:enforce_permissions", "bool": true, "set": true}"#;
    let off = br#"{"option": "ecosystem:enforce_permissions", "bool": false, "set": false}"#;
    assert_eq!(parse_enforced(on), Some(true));
    assert_eq!(parse_enforced(off), Some(false));
}

#[test]
fn an_int_reply_is_enforced_when_nonzero() {
    assert_eq!(
        parse_enforced(br#"{"option": "x", "int": 1, "set": true}"#),
        Some(true)
    );
    assert_eq!(
        parse_enforced(br#"{"option": "x", "int": 0, "set": false}"#),
        Some(false)
    );
}

#[test]
fn a_hyprland_without_the_option_enforces_nothing() {
    assert_eq!(parse_enforced(b"no such option\n"), Some(false));
}

#[test]
fn an_unreadable_reply_is_not_taken_as_allowed() {
    assert_eq!(parse_enforced(b"bool: true\nset: true\n"), None);
    assert_eq!(parse_enforced(br#"{"option": "x", "str": "yes"}"#), None);
    assert_eq!(parse_enforced(&[0xff, 0xfe]), None);
}
