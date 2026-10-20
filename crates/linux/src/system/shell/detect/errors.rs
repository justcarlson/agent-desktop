use agent_desktop_core::{AdapterError, ErrorCode};
use serde_json::json;

use super::{Fact, ShellEnvironment};

pub(super) fn unreadable(what: &str, why: &str, key: &str) -> AdapterError {
    AdapterError::new(ErrorCode::Timeout, format!("The {what} could not be read"))
        .with_suggestion("Retry the command; report the detail if it persists")
        .with_details(json!({ key: "unreadable", "retryable": true, "detail": why }))
}

pub(super) fn tree_error(env: &ShellEnvironment) -> Result<(), AdapterError> {
    match &env.tree {
        Fact::Present => Ok(()),
        Fact::Absent(why) => Err(AdapterError::new(
            ErrorCode::AppNotFound,
            "The desktop shell is not on the accessibility registry",
        )
        .with_suggestion(
            "Restart the shell session so it registers again; do not toggle accessibility settings",
        )
        .with_details(json!({ "shell_tree": "absent", "detail": why }))),
        Fact::Unreadable(why) => Err(unreadable("shell accessibility tree", why, "shell_tree")),
    }
}
