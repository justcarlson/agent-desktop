//! Live reads of the bound instance's clients, the clients that fill their
//! workspace, and its desktop state.

use agent_desktop_core::{AdapterError, Deadline, Point};

use super::client_fullscreen::parse_filling;
use super::client_parse::{parse_active, parse_clients};
use super::desktop_state::{DesktopState, parse_cursor, parse_monitors};
use super::hypr_client::HyprClient;
use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;
use crate::system::hyprland::ipc::bound_request::bound_request;

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

/// The address and workspace of every client of the bound instance that
/// fills its workspace, or `None` when the reply does not say.
pub(crate) fn read_filling(
    binding: &HyprlandBinding,
    deadline: Deadline,
) -> Result<Option<Vec<(u64, String)>>, AdapterError> {
    let body = bound_request(binding, CLIENTS_COMMAND, deadline)?;
    Ok(parse_filling(&body))
}

/// The active window and every monitor's workspaces of the bound instance.
/// The cursor is left unread; `read_cursor` reads it for a caller that needs
/// it.
pub(crate) fn read_desktop(
    binding: &HyprlandBinding,
    deadline: Deadline,
) -> Result<DesktopState, AdapterError> {
    let active = bound_request(binding, ACTIVE_COMMAND, deadline)?;
    let active = parse_active(&active).ok_or_else(|| unreadable("active window"))?;
    let monitors = bound_request(binding, MONITORS_COMMAND, deadline)?;
    let monitors = parse_monitors(&monitors).ok_or_else(|| unreadable("monitor list"))?;
    Ok(DesktopState {
        active,
        monitors,
        cursor: None,
    })
}

/// The cursor position of the bound instance in global compositor
/// coordinates.
pub(crate) fn read_cursor(
    binding: &HyprlandBinding,
    deadline: Deadline,
) -> Result<Point, AdapterError> {
    let cursor = bound_request(binding, CURSOR_COMMAND, deadline)?;
    parse_cursor(&cursor).ok_or_else(|| unreadable("cursor position"))
}

fn unreadable(what: &str) -> AdapterError {
    AdapterError::internal(format!("The Hyprland {what} could not be read"))
        .with_suggestion("Retry once the compositor answers")
}
