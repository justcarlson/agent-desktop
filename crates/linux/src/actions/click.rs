use agent_desktop_core::{ActionResult, AdapterError};

use super::context::Target;

pub(crate) fn click(_target: &Target<'_>) -> Result<ActionResult, AdapterError> {
    Err(AdapterError::not_supported("click"))
}

pub(crate) fn right_click(_target: &Target<'_>) -> Result<ActionResult, AdapterError> {
    Err(AdapterError::not_supported("right-click"))
}

#[cfg(test)]
#[path = "click_tests.rs"]
mod tests;
