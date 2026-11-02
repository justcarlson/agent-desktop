//! Where each monitor's top-left corner sits in the layout, from a
//! `j/monitors` reply. Client places are in the same layout coordinates.

use serde_json::Value;

/// One monitor's id, as clients name it, and its layout origin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OutputOrigin {
    pub(crate) id: i64,
    pub(crate) at: (i64, i64),
}

/// Every monitor of a `j/monitors` reply, or `None` when one lacks its id
/// or origin.
pub(crate) fn parse_origins(body: &[u8]) -> Option<Vec<OutputOrigin>> {
    let monitors: Vec<Value> = serde_json::from_slice(body).ok()?;
    monitors.iter().map(origin_of_monitor).collect()
}

/// The origin of the monitor `id` names among `origins`.
pub(crate) fn origin_of(origins: &[OutputOrigin], id: Option<i64>) -> Option<(i64, i64)> {
    let id = id?;
    origins
        .iter()
        .find(|origin| origin.id == id)
        .map(|origin| origin.at)
}

fn origin_of_monitor(monitor: &Value) -> Option<OutputOrigin> {
    Some(OutputOrigin {
        id: monitor.get("id")?.as_i64()?,
        at: (monitor.get("x")?.as_i64()?, monitor.get("y")?.as_i64()?),
    })
}

#[cfg(test)]
#[path = "output_origin_tests.rs"]
mod tests;
