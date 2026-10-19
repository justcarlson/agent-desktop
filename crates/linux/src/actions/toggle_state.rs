use agent_desktop_core::{ActionResult, AdapterError};

use super::context::Target;

pub(crate) fn toggle(
    _target: &Target<'_>,
    _desired: Option<bool>,
) -> Result<ActionResult, AdapterError> {
    Err(AdapterError::not_supported("toggle"))
}

#[cfg(test)]
#[path = "toggle_state_tests.rs"]
mod tests;
