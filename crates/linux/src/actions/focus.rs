use agent_desktop_core::{ActionResult, AdapterError, InteractionPolicy};

use super::context::Target;

pub(crate) fn focus(
    _target: &Target<'_>,
    _policy: InteractionPolicy,
) -> Result<ActionResult, AdapterError> {
    Err(AdapterError::not_supported("focus"))
}

#[cfg(test)]
#[path = "focus_tests.rs"]
mod tests;
