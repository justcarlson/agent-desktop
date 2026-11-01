use super::*;

#[test]
fn the_probe_reply_names_the_dialect() {
    assert_eq!(dialect_of("ok"), Some(ConfigDialect::Lua));
    assert_eq!(dialect_of("ok\n"), Some(ConfigDialect::Lua));
    assert_eq!(
        dialect_of("eval is only supported with the lua config manager"),
        Some(ConfigDialect::Legacy)
    );
}

#[test]
fn any_other_reply_names_no_dialect() {
    assert_eq!(dialect_of("unknown request"), None);
    assert_eq!(dialect_of(""), None);
    assert_eq!(dialect_of("error: attempt to call a nil value"), None);
}
