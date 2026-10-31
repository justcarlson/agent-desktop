//! Live reads of the bound instance's clients and desktop state.

use agent_desktop_core::{AdapterError, Deadline};

use super::client_parse::{parse_active, parse_clients};
use super::desktop_state::{DesktopState, parse_cursor, parse_monitors};
use super::hypr_client::HyprClient;
use crate::system::hyprland::bound_request::bound_request;
use crate::system::hyprland::hyprland_binding::HyprlandBinding;

const CLIENTS_COMMAND: &str = "j/clients";
const ACTIVE_COMMAND: &str = "j/activewindow";
const MONITORS_COMMAND: &str = "j/monitors";
const CURSOR_COMMAND: &str = "j/cursorpos";

/// Every client of the bound instance.
pub(crate) fn read_clients(
    binding: &HyprlandBinding,
    deadline: Deadline,
) -> Result<Vec<HyprClient>, AdapterError> {
    let body = bound_request(binding, CLIENTS_COMMAND, deadline)?;
    parse_clients(&body).ok_or_else(|| unreadable("client list"))
}

/// The active window, every monitor's workspaces and the cursor of the
/// bound instance.
pub(crate) fn read_desktop(
    binding: &HyprlandBinding,
    deadline: Deadline,
) -> Result<DesktopState, AdapterError> {
    let active = bound_request(binding, ACTIVE_COMMAND, deadline)?;
    let active = parse_active(&active).ok_or_else(|| unreadable("active window"))?;
    let monitors = bound_request(binding, MONITORS_COMMAND, deadline)?;
    let monitors = parse_monitors(&monitors).ok_or_else(|| unreadable("monitor list"))?;
    let cursor = bound_request(binding, CURSOR_COMMAND, deadline)?;
    let cursor = parse_cursor(&cursor).ok_or_else(|| unreadable("cursor position"))?;
    Ok(DesktopState {
        active,
        monitors,
        cursor: Some(cursor),
    })
}

fn unreadable(what: &str) -> AdapterError {
    AdapterError::internal(format!("The Hyprland {what} could not be read"))
        .with_suggestion("Retry once the compositor answers")
}
