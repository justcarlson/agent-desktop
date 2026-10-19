use super::*;

fn names(targets: &[Target]) -> Vec<&str> {
    targets.iter().map(|target| target.name.as_str()).collect()
}

#[test]
fn invalid_utf8_is_an_error_and_never_replaced() {
    for target in [UTF8_STRING, TEXT_UTF8, TEXT_PLAIN] {
        let error = decode_text(target, &[0x68, 0xff, 0x69]).expect_err("invalid UTF-8");
        assert_eq!(error.code, ErrorCode::ActionFailed, "{target}");
    }
}

#[test]
fn string_target_is_latin1() {
    assert_eq!(
        decode_text(STRING, &[0x63, 0x61, 0x66, 0xe9]).expect("latin1"),
        "café"
    );
}

#[test]
fn utf8_target_keeps_multibyte_text() {
    assert_eq!(
        decode_text(UTF8_STRING, "café ☃".as_bytes()).expect("utf8"),
        "café ☃"
    );
}

#[test]
fn a_target_that_is_not_text_is_refused() {
    assert!(decode_text("image/png", b"x").is_err());
}

#[test]
fn text_within_latin1_serves_all_four_targets() {
    let targets = encode_text_targets("café");
    assert_eq!(
        names(&targets),
        [UTF8_STRING, TEXT_UTF8, TEXT_PLAIN, STRING]
    );
    assert_eq!(targets[3].bytes, [0x63, 0x61, 0x66, 0xe9]);
    assert_eq!(targets[0].bytes, "café".as_bytes());
}

#[test]
fn a_character_above_u00ff_omits_the_string_target() {
    let targets = encode_text_targets("snow ☃");
    assert_eq!(names(&targets), [UTF8_STRING, TEXT_UTF8, TEXT_PLAIN]);
}

#[test]
fn the_empty_string_is_present_and_decodes_as_present_empty() {
    let targets = encode_text_targets("");
    assert_eq!(targets.len(), 4);
    for target in &targets {
        assert_eq!(decode_text(&target.name, &target.bytes).expect("empty"), "");
    }
}
