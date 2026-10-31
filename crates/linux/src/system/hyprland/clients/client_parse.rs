//! Parsers for the `j/clients` and `j/activewindow` replies. A client the
//! parser cannot place by address and process makes the whole reply
//! unusable, because a missing client could be the only candidate for a
//! frame.

use agent_desktop_core::Rect;
use serde_json::Value;

use super::client_workspace::ClientWorkspace;
use super::hypr_client::HyprClient;

/// Every client of a `j/clients` reply, in the compositor's order.
pub(crate) fn parse_clients(body: &[u8]) -> Option<Vec<HyprClient>> {
    let clients: Vec<Value> = serde_json::from_slice(body).ok()?;
    clients.iter().map(client_of).collect()
}

/// The active window's address from `j/activewindow`. An empty object means
/// no window is active; anything else that is not a window is `None`.
pub(crate) fn parse_active(body: &[u8]) -> Option<Option<u64>> {
    let active: Value = serde_json::from_slice(body).ok()?;
    let object = active.as_object()?;
    match object.get("address") {
        None => Some(None),
        Some(address) => parse_address(address.as_str()?).map(Some),
    }
}

/// A Hyprland address such as `0x55e7761d2e70`.
pub(crate) fn parse_address(text: &str) -> Option<u64> {
    let digits = text.strip_prefix("0x")?;
    if digits.is_empty() {
        return None;
    }
    u64::from_str_radix(digits, 16).ok()
}

fn client_of(client: &Value) -> Option<HyprClient> {
    let pair = |key: &str| -> Option<(f64, f64)> {
        let values = client.get(key)?.as_array()?;
        Some((values.first()?.as_f64()?, values.get(1)?.as_f64()?))
    };
    let text = |key: &str| client.get(key).and_then(Value::as_str).unwrap_or_default();
    let flag = |key: &str| client.get(key).and_then(Value::as_bool).unwrap_or(false);
    let (x, y) = pair("at")?;
    let (width, height) = pair("size")?;
    Some(HyprClient {
        address: parse_address(client.get("address")?.as_str()?)?,
        pid: u32::try_from(client.get("pid")?.as_i64()?).ok()?,
        title: text("title").to_owned(),
        class: text("class").to_owned(),
        geometry: Rect {
            x,
            y,
            width,
            height,
        },
        workspace: ClientWorkspace {
            name: client
                .get("workspace")
                .and_then(|workspace| workspace.get("name"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            monitor: client.get("monitor").and_then(Value::as_i64),
        },
        shown: flag("mapped") && !flag("hidden"),
    })
}

#[cfg(test)]
#[path = "client_parse_tests.rs"]
mod tests;
