//! The one place a headed Hyprland command hides the agent's cursors: after
//! it holds the interaction lease, before receiver evidence, the focus gate
//! or any event.

use agent_desktop_core::{AdapterError, Deadline, DeliverySemantics};
use serde_json::json;

use super::show_on_drop::ShowOnDrop;
use crate::system::hyprland::bind::routed_binding::routed_binding;
use crate::system::hyprland::cursor_context::CursorContext;
use crate::system::hyprland::cursor_hider::CursorHider;

/// Runs `send` with `hider`'s cursors hidden. A failed hide returns before
/// `send` runs, as not delivered; `show` runs on every exit after a
/// successful hide, including a panic in `send`.
pub(crate) fn with_cursors_hidden<T>(
    hider: &mut dyn CursorHider,
    deadline: Deadline,
    send: impl FnOnce() -> Result<T, AdapterError>,
) -> Result<T, AdapterError> {
    let _ = (hider, deadline, ShowOnDrop::new);
    send()
}

/// [`with_cursors_hidden`] with the factory's hider on the bound Hyprland
/// route, and `send` alone on any other route.
pub(crate) fn hidden_on_route<T>(
    deadline: Deadline,
    send: impl FnOnce() -> Result<T, AdapterError>,
) -> Result<T, AdapterError> {
    match routed_binding().map_err(nothing_sent)? {
        Some(binding) => {
            let mut hider =
                crate::system::hyprland::cursor_hider(&CursorContext { binding: &binding });
            with_cursors_hidden(hider.as_mut(), deadline, send)
        }
        None => send(),
    }
}

fn nothing_sent(error: AdapterError) -> AdapterError {
    let mut details = match error.details.clone() {
        Some(serde_json::Value::Object(details)) => details,
        Some(other) => serde_json::Map::from_iter([("cause".to_owned(), other)]),
        None => serde_json::Map::new(),
    };
    details.insert("physical_delivery_started".to_owned(), json!(false));
    error
        .with_details(serde_json::Value::Object(details))
        .with_disposition(DeliverySemantics::not_delivered())
}

#[cfg(test)]
#[path = "cursor_guard_tests.rs"]
mod tests;
