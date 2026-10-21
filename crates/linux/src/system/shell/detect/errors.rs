use agent_desktop_core::{AdapterError, ErrorCode};
use serde_json::json;

use super::Fact;

pub(super) fn unreadable(what: &str, why: &str, key: &str) -> AdapterError {
    AdapterError::new(ErrorCode::Timeout, format!("The {what} could not be read"))
        .with_suggestion("Retry the command; report the detail if it persists")
        .with_details(json!({ key: "unreadable", "retryable": true, "detail": why }))
}

pub(super) fn tree_error(tree: &Fact) -> Result<(), AdapterError> {
    match tree {
        Fact::Present => Ok(()),
        Fact::Absent(why) => Err(crate::system::shell::identity::tree_absent(why)),
        Fact::Unreadable(why) => Err(unreadable("shell accessibility tree", why, "shell_tree")),
    }
}
