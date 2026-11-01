use agent_desktop_core::{DeliverySemantics, ErrorCode};

use super::refuse_control;
use crate::input::keyboard::keysym_step::KeysymStep;
use crate::input::keyboard::text_plan::plan_text_keysyms;

fn verdict(text: &str) -> Result<(), agent_desktop_core::AdapterError> {
    let chunks = plan_text_keysyms(text);
    let slices: Vec<&[KeysymStep]> = chunks.iter().map(Vec::as_slice).collect();
    refuse_control(&slices)
}

#[test]
fn text_newline_tab_and_any_script_pass() {
    for text in ["Hello, World!", "a\nb\tc", "привет", "é€ü"] {
        assert!(verdict(text).is_ok(), "{text:?} was refused");
    }
}

#[test]
fn a_control_character_is_refused_before_any_key() {
    for text in ["a\u{7}b", "\r", "x\u{1b}", "\u{7f}", "\u{85}"] {
        let error = verdict(text).expect_err("refused");
        assert_eq!(error.code, ErrorCode::ActionNotSupported);
        assert_eq!(error.disposition, DeliverySemantics::not_delivered());
        let details = error.details.expect("details");
        assert_eq!(details["cause"], "character_outside_layout");
        assert_eq!(details["physical_delivery_started"], false);
    }
}
