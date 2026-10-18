use std::collections::HashMap;

use agent_desktop_core::{AdapterError, Deadline};
use zbus::connection::Builder;
use zbus::zvariant::OwnedValue;

use super::bus_deadline::within;

const SERVICE: &str = "org.gnome.Mutter.DisplayConfig";
const PATH: &str = "/org/gnome/Mutter/DisplayConfig";
const INTERFACE: &str = "org.gnome.Mutter.DisplayConfig";
const LOGICAL_LAYOUT: u32 = 1;

/// One logical monitor as Mutter reports it. The reply carries position,
/// scale, the primary flag and the connectors driving it, but no size; the
/// size is derived from the driving monitor's current mode.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LogicalMonitor {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) width: i32,
    pub(crate) height: i32,
    pub(crate) scale: f64,
    pub(crate) primary: bool,
    pub(crate) connectors: Vec<String>,
}

type Properties = HashMap<String, OwnedValue>;
type Spec = (String, String, String, String);
type Mode = (String, i32, i32, f64, f64, Vec<f64>, Properties);
type Physical = (Spec, Vec<Mode>, Properties);
type Logical = (i32, i32, f64, u32, bool, Vec<Spec>, Properties);
type Reply = (u32, Vec<Physical>, Vec<Logical>, Properties);

fn flag(properties: &Properties, key: &str) -> Option<bool> {
    properties
        .get(key)
        .and_then(|value| value.downcast_ref::<bool>().ok())
}

fn logical_layout(properties: &Properties) -> bool {
    properties
        .get("layout-mode")
        .and_then(|value| value.downcast_ref::<u32>().ok())
        == Some(LOGICAL_LAYOUT)
}

fn current_size(modes: &[Mode]) -> Option<(i32, i32)> {
    modes
        .iter()
        .find(|mode| flag(&mode.6, "is-current") == Some(true))
        .map(|mode| (mode.1, mode.2))
}

fn logical_size(monitors: &[Physical], specs: &[Spec], scale: f64, logical: bool) -> (i32, i32) {
    let size = specs.iter().find_map(|spec| {
        let monitor = monitors.iter().find(|monitor| monitor.0.0 == spec.0)?;
        current_size(&monitor.1)
    });
    match size {
        Some((width, height)) if logical && scale > 0.0 => (
            (f64::from(width) / scale).round() as i32,
            (f64::from(height) / scale).round() as i32,
        ),
        Some(size) => size,
        None => (0, 0),
    }
}

fn assemble(reply: Reply) -> Vec<LogicalMonitor> {
    let (_, monitors, logical, properties) = reply;
    let layout_is_logical = logical_layout(&properties);
    logical
        .into_iter()
        .map(|(x, y, scale, _, primary, specs, _)| {
            let (width, height) = logical_size(&monitors, &specs, scale, layout_is_logical);
            LogicalMonitor {
                x,
                y,
                width,
                height,
                scale,
                primary,
                connectors: specs.into_iter().map(|spec| spec.0).collect(),
            }
        })
        .collect()
}

async fn get_current_state() -> Option<Vec<LogicalMonitor>> {
    let session = Builder::session().ok()?.build().await.ok()?;
    let reply = session
        .call_method(Some(SERVICE), PATH, Some(INTERFACE), "GetCurrentState", &())
        .await
        .ok()?;
    let body = reply.body().deserialize::<Reply>().ok()?;
    Some(assemble(body))
}

/// The compositor's logical monitor list, or `None` when the session bus has
/// no Mutter answering it. Only the deadline itself is an error.
pub(crate) fn logical_monitors(
    deadline: Deadline,
) -> Result<Option<Vec<LogicalMonitor>>, AdapterError> {
    within(deadline, async { Ok(get_current_state().await) })
}
