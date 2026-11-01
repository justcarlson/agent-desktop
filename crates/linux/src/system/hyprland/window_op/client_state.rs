//! The window-op state of every client in a `j/clients` reply: where it
//! is, its size, whether it floats, its fullscreen mode and its workspace.

use serde_json::Value;

use crate::system::hyprland::clients::client_parse::parse_address;

/// One client's place and state. `fullscreen` is Hyprland's internal mode:
/// 0 none, 1 maximized, 2 fullscreen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ClientState {
    pub(crate) address: u64,
    pub(crate) at: (i64, i64),
    pub(crate) size: (i64, i64),
    pub(crate) floating: bool,
    pub(crate) fullscreen: i64,
    pub(crate) workspace: String,
}

impl ClientState {
    pub(crate) fn to_json(&self) -> Value {
        serde_json::json!({
            "x": self.at.0,
            "y": self.at.1,
            "width": self.size.0,
            "height": self.size.1,
            "floating": self.floating,
            "fullscreen": self.fullscreen,
            "workspace": self.workspace,
        })
    }
}

/// Every client of a `j/clients` reply, or `None` when one client is
/// unreadable.
pub(crate) fn parse_states(body: &[u8]) -> Option<Vec<ClientState>> {
    let clients: Vec<Value> = serde_json::from_slice(body).ok()?;
    clients.iter().map(state_of).collect()
}

fn state_of(client: &Value) -> Option<ClientState> {
    Some(ClientState {
        address: parse_address(client.get("address")?.as_str()?)?,
        at: pair(client.get("at")?)?,
        size: pair(client.get("size")?)?,
        floating: client.get("floating")?.as_bool()?,
        fullscreen: client.get("fullscreen")?.as_i64()?,
        workspace: client.get("workspace")?.get("name")?.as_str()?.to_owned(),
    })
}

fn pair(value: &Value) -> Option<(i64, i64)> {
    let items = value.as_array()?;
    match items.as_slice() {
        [first, second] => Some((number(first)?, number(second)?)),
        _ => None,
    }
}

fn number(value: &Value) -> Option<i64> {
    value.as_i64().or_else(|| {
        value
            .as_f64()
            .filter(|n| n.is_finite())
            .map(|n| n.round() as i64)
    })
}

#[cfg(test)]
#[path = "client_state_tests.rs"]
mod tests;
