use agent_desktop_core::{ActionResult, AdapterError};

use super::context::Target;

pub(crate) fn select(
    _target: &Target<'_>,
    _display_value: &str,
) -> Result<ActionResult, AdapterError> {
    Err(AdapterError::not_supported("select"))
}

#[cfg(test)]
#[path = "select_tests.rs"]
mod tests;
