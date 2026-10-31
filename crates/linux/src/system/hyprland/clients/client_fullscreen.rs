//! Which clients of a `j/clients` reply fill their workspace. Hyprland
//! reports `fullscreen` as 1 for a maximized client and 2 for a fullscreen
//! one, and composes no other client of that workspace while one does.

use serde_json::Value;

use super::client_parse::parse_address;

/// The address and workspace name of every client that fills its
/// workspace. A client whose address or fullscreen mode is unreadable makes
/// the whole reply unusable.
pub(crate) fn parse_filling(body: &[u8]) -> Option<Vec<(u64, String)>> {
    let clients: Vec<Value> = serde_json::from_slice(body).ok()?;
    let mut filling = Vec::new();
    for client in &clients {
        let address = parse_address(client.get("address")?.as_str()?)?;
        if client.get("fullscreen")?.as_i64()? > 0 {
            let workspace = client
                .get("workspace")
                .and_then(|workspace| workspace.get("name"))
                .and_then(Value::as_str)
                .unwrap_or_default();
            filling.push((address, workspace.to_owned()));
        }
    }
    Some(filling)
}

#[cfg(test)]
#[path = "client_fullscreen_tests.rs"]
mod tests;
