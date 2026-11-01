//! Parsers for the `j/clients` and `j/layers` replies as receiver evidence.
//! An entry the parser cannot read makes the whole reply unusable, because
//! the unread entry could be the cover.

use agent_desktop_core::Rect;
use serde_json::Value;

use super::layer_info::LayerInfo;
use super::stack_client::StackClient;
use super::stacking::Stacking;
use crate::system::hyprland::clients::client_parse::parse_address;
use crate::system::hyprland::clients::client_workspace::ClientWorkspace;

/// Every client of a `j/clients` reply with its stacking state.
pub(crate) fn parse_stack_clients(body: &[u8]) -> Option<Vec<StackClient>> {
    let clients: Vec<Value> = serde_json::from_slice(body).ok()?;
    clients.iter().map(client_of).collect()
}

/// Every layer surface of a `j/layers` reply, on every output and level.
pub(crate) fn parse_layers(body: &[u8]) -> Option<Vec<LayerInfo>> {
    let outputs: serde_json::Map<String, Value> = serde_json::from_slice(body).ok()?;
    let mut layers = Vec::new();
    for (output, entry) in &outputs {
        for (level, surfaces) in entry.get("levels")?.as_object()? {
            let level: u8 = level.parse().ok()?;
            for surface in surfaces.as_array()? {
                layers.push(layer_of(surface, level, output)?);
            }
        }
    }
    Some(layers)
}

fn client_of(client: &Value) -> Option<StackClient> {
    let pair = |key: &str| -> Option<(f64, f64)> {
        let values = client.get(key)?.as_array()?;
        Some((values.first()?.as_f64()?, values.get(1)?.as_f64()?))
    };
    let flag = |key: &str| client.get(key).and_then(Value::as_bool);
    let (x, y) = pair("at")?;
    let (width, height) = pair("size")?;
    Some(StackClient {
        address: parse_address(client.get("address")?.as_str()?)?,
        rect: Rect {
            x,
            y,
            width,
            height,
        },
        workspace: ClientWorkspace {
            name: client.get("workspace")?.get("name")?.as_str()?.to_owned(),
            monitor: client.get("monitor").and_then(Value::as_i64),
        },
        stacking: Stacking {
            floating: flag("floating")?,
            pinned: flag("pinned")?,
            fullscreen: client.get("fullscreen")?.as_i64()? > 0,
            over_fullscreen: None,
        },
        shown: flag("mapped")? && !flag("hidden").unwrap_or(false),
    })
}

fn layer_of(surface: &Value, level: u8, output: &str) -> Option<LayerInfo> {
    let number = |key: &str| surface.get(key).and_then(Value::as_f64);
    Some(LayerInfo {
        namespace: surface.get("namespace")?.as_str()?.to_owned(),
        pid: surface
            .get("pid")
            .and_then(Value::as_u64)
            .and_then(|pid| u32::try_from(pid).ok()),
        rect: Rect {
            x: number("x")?,
            y: number("y")?,
            width: number("w")?,
            height: number("h")?,
        },
        level,
        output: output.to_owned(),
    })
}

#[cfg(test)]
#[path = "stack_parse_tests.rs"]
mod tests;
