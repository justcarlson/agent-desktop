//! The layout the seat owner types in, read from the bound compositor's
//! `j/devices` reply. A client without keyboard focus never receives the
//! owner's `modifiers` event, so the layout index cannot come from Wayland.
//! The main keyboard's `active_layout_index` names it; a reply without that
//! field is matched by the main keyboard's `active_keymap` name against the
//! owner keymap's layout names. A keymap with one layout needs neither.

use agent_desktop_core::{AdapterError, Deadline};
use serde_json::{Value, json};

use crate::input::wayland::keymap::refusal::keymap_refused;
use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;
use crate::system::hyprland::ipc::bound_request::bound_request;

const DEVICES_COMMAND: &str = "j/devices";

/// The owner's active layout index on the bound instance, for a keymap whose
/// layouts are named `layouts` in index order.
pub(crate) fn read_active_layout(
    binding: &HyprlandBinding,
    layouts: &[String],
    deadline: Deadline,
) -> Result<u32, AdapterError> {
    let body = bound_request(binding, DEVICES_COMMAND, deadline)?;
    active_layout(&body, layouts)
}

/// The active layout index a `j/devices` reply names, checked against the
/// keymap's `layouts`.
pub(crate) fn active_layout(devices: &[u8], layouts: &[String]) -> Result<u32, AdapterError> {
    let count = u32::try_from(layouts.len()).unwrap_or(u32::MAX);
    let main = main_keyboard(devices);
    let index = main
        .as_ref()
        .and_then(|keyboard| keyboard.get("active_layout_index"))
        .and_then(Value::as_u64)
        .and_then(|index| u32::try_from(index).ok());
    if let Some(index) = index {
        return if index < count {
            Ok(index)
        } else {
            Err(unknown(layouts, main.as_ref()))
        };
    }
    let named = main
        .as_ref()
        .and_then(|keyboard| keyboard.get("active_keymap"))
        .and_then(Value::as_str)
        .map(|name| {
            layouts
                .iter()
                .enumerate()
                .filter(|(_, layout)| layout.as_str() == name)
                .map(|(index, _)| index)
                .collect::<Vec<_>>()
        });
    match named.as_deref() {
        Some([index]) => Ok(u32::try_from(*index).unwrap_or(u32::MAX)),
        _ if count == 1 => Ok(0),
        _ => Err(unknown(layouts, main.as_ref())),
    }
}

fn main_keyboard(devices: &[u8]) -> Option<Value> {
    let reply: Value = serde_json::from_slice(devices).ok()?;
    let keyboards = reply.get("keyboards")?.as_array()?;
    keyboards
        .iter()
        .find(|keyboard| keyboard.get("main").and_then(Value::as_bool) == Some(true))
        .cloned()
}

fn unknown(layouts: &[String], main: Option<&Value>) -> AdapterError {
    let active = main
        .and_then(|keyboard| keyboard.get("active_keymap"))
        .cloned();
    keymap_refused(
        "The active keyboard layout of this Hyprland session could not be read",
        "active_layout_unknown",
        json!({ "layouts": layouts, "active_keymap": active }),
    )
}

#[cfg(test)]
#[path = "active_layout_tests.rs"]
mod tests;
