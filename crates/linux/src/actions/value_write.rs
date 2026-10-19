use agent_desktop_core::{ActionResult, AdapterError};

use super::context::Target;

pub(crate) fn set_value(_target: &Target<'_>, _text: &str) -> Result<ActionResult, AdapterError> {
    Err(AdapterError::not_supported("set-value"))
}

pub(crate) fn clear(_target: &Target<'_>) -> Result<ActionResult, AdapterError> {
    Err(AdapterError::not_supported("clear"))
}

pub(crate) fn type_text(_target: &Target<'_>, _text: &str) -> Result<ActionResult, AdapterError> {
    Err(AdapterError::not_supported("type"))
}

#[cfg(test)]
#[path = "value_write_tests.rs"]
mod tests;
