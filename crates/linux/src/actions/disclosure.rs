use agent_desktop_core::{ActionResult, AdapterError};

use super::context::Target;

pub(crate) fn set_expanded(
    _target: &Target<'_>,
    _expanded: bool,
) -> Result<ActionResult, AdapterError> {
    Err(AdapterError::not_supported("expand"))
}

#[cfg(test)]
#[path = "disclosure_tests.rs"]
mod tests;
