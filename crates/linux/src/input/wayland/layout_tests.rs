use super::*;

fn press(keysym: u32) -> KeysymStep {
    KeysymStep { keysym, down: true }
}

#[test]
fn printable_ascii_function_and_vendor_keys_are_reachable() {
    for keysym in [0x20, 0x41, 0x61, 0x7e, 0xff0d, 0xffe1, 0xfe03, 0x1008_ff11] {
        assert!(reachable(keysym), "{keysym:x} must be reachable");
    }
}

#[test]
fn latin1_euro_and_unicode_form_keysyms_are_not_reachable() {
    for keysym in [0xe9, 0xa3, 0x20ac, 0x0100_4f60, 0x0101_f600, 0x1f] {
        assert!(!reachable(keysym), "{keysym:x} must be refused");
    }
}

#[test]
fn one_unreachable_key_refuses_the_whole_sequence_before_any_send() {
    let keys = [press(0x68), press(0x0100_4f60), press(0x69)];
    let error = refuse_unreachable(&keys).expect_err("the sequence is refused");
    assert_eq!(error.code, ErrorCode::ActionNotSupported);
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
    let details = error.details.as_ref().expect("details");
    assert_eq!(details["physical_delivery_started"], false);
    assert_eq!(details["cause"], "character_outside_layout");
    assert!(
        error
            .suggestion
            .as_deref()
            .is_some_and(|text| text.contains("set-value"))
    );
    let rendered = format!("{error:?}");
    assert!(!rendered.contains("4f60") && !rendered.contains("你"));
}

#[test]
fn a_reachable_sequence_is_not_refused() {
    assert!(refuse_unreachable(&[press(0xffe3), press(0x61)]).is_ok());
}
