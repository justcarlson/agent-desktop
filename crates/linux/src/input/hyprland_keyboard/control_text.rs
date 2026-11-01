//! The text a virtual keyboard refuses before it opens: control characters
//! other than newline and tab. The extended keymap could carry them as
//! Unicode keysyms, and an application may act on them, so they are refused
//! as on the portal route.

use agent_desktop_core::{AdapterError, DeliverySemantics, ErrorCode};
use serde_json::json;

use crate::input::keyboard::key_names::UNICODE_KEYSYM_BASE;
use crate::input::keyboard::keysym_step::KeysymStep;

const C0_CONTROLS: std::ops::Range<u32> = 0x00..0x20;
const DELETE_AND_C1_CONTROLS: std::ops::Range<u32> = 0x7f..0xa0;

fn is_control(keysym: u32) -> bool {
    keysym
        .checked_sub(UNICODE_KEYSYM_BASE)
        .is_some_and(|code| C0_CONTROLS.contains(&code) || DELETE_AND_C1_CONTROLS.contains(&code))
}

/// Refuses `chunks` when any key is a control character, before any key is sent.
pub(crate) fn refuse_control(chunks: &[&[KeysymStep]]) -> Result<(), AdapterError> {
    if !chunks
        .iter()
        .flat_map(|chunk| chunk.iter())
        .any(|key| is_control(key.keysym))
    {
        return Ok(());
    }
    Err(AdapterError::new(
        ErrorCode::ActionNotSupported,
        "A control character in the text has no key a virtual keyboard may send",
    )
    .with_suggestion("Use the semantic set-value on the element, or press the key by name")
    .with_details(json!({
        "physical_delivery_started": false,
        "cause": "character_outside_layout",
    }))
    .with_disposition(DeliverySemantics::not_delivered()))
}

#[cfg(test)]
#[path = "control_text_tests.rs"]
mod tests;
