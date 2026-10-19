use agent_desktop_core::AdapterError;

use super::context::Target;
use super::mutation::WriteReply;

#[expect(dead_code, reason = "no caller yet")]
pub(crate) fn resolve_verb(_names: &[String], _verbs: &[&str]) -> Option<i32> {
    None
}

#[expect(dead_code, reason = "no caller yet")]
pub(crate) fn do_action(
    _target: &Target<'_>,
    _names: &[String],
    _verbs: &[&str],
) -> Result<WriteReply, AdapterError> {
    Ok(WriteReply::Absent)
}

#[cfg(test)]
#[path = "action_call_tests.rs"]
mod tests;
