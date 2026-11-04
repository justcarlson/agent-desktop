//! The parser for the `j/layers` reply. A layer the parser cannot read
//! makes the whole reply unusable, because the unread layer could be an
//! agent cursor.

use agent_desktop_core::Rect;
use serde_json::Value;

use super::layer_info::LayerInfo;

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
