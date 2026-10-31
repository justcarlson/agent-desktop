//! Whether the bound Hyprland enforces its permission system. With
//! `ecosystem:enforce_permissions` on, Hyprland withholds every screen and
//! toplevel frame from a client it has not allowed, and it says nothing:
//! no failed event, no dialog. Measured on Hyprland 0.56.2 and 0.57.0, a
//! client named by an allow rule was refused as well, so capture asks the
//! compositor before it sends anything.

use agent_desktop_core::{AdapterError, Deadline};
use serde_json::Value;

use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;
use crate::system::hyprland::ipc::bound_request::bound_request;

pub(crate) const ENFORCE_OPTION: &str = "ecosystem:enforce_permissions";
const UNKNOWN_OPTION: &str = "no such option";

/// Reads the option from the bound instance. A reply that cannot be read
/// is an error, never a guess that capture is allowed.
pub(crate) fn read_enforced(
    binding: &HyprlandBinding,
    deadline: Deadline,
) -> Result<bool, AdapterError> {
    let body = bound_request(binding, &format!("j/getoption {ENFORCE_OPTION}"), deadline)?;
    parse_enforced(&body).ok_or_else(|| {
        AdapterError::internal(format!(
            "The Hyprland option {ENFORCE_OPTION} could not be read"
        ))
        .with_suggestion("Retry once the compositor answers")
    })
}

/// Parses a `j/getoption` reply. Hyprland 0.56.2 and 0.57.0 report the
/// option as a `bool`; releases on the older config parser report a boolean
/// as an `int`. A Hyprland without the option has no permission system, so
/// it enforces nothing.
pub(crate) fn parse_enforced(body: &[u8]) -> Option<bool> {
    let text = std::str::from_utf8(body).ok()?.trim();
    if text == UNKNOWN_OPTION {
        return Some(false);
    }
    let reply: Value = serde_json::from_str(text).ok()?;
    if let Some(flag) = reply.get("bool").and_then(Value::as_bool) {
        return Some(flag);
    }
    reply
        .get("int")
        .and_then(Value::as_i64)
        .map(|value| value != 0)
}

#[cfg(test)]
#[path = "enforce_option_tests.rs"]
mod tests;
