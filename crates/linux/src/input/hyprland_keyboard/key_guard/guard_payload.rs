//! What a keyboard command hands its key guard on standard input: the bound
//! Hyprland instance, the command's own process identity and the layout the
//! command types in.

use std::path::PathBuf;

use serde_json::{Value, json};

use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;

/// The guard's whole startup input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GuardPayload {
    pub(crate) binding: HyprlandBinding,
    pub(crate) command_pid: u32,
    pub(crate) command_instance: String,
    pub(crate) layout: u32,
}

impl GuardPayload {
    /// The payload as JSON, or `None` when a socket path is not UTF-8.
    pub(crate) fn to_json(&self) -> Option<Value> {
        Some(json!({
            "signature": self.binding.signature,
            "ipc_socket": self.binding.ipc_socket.to_str()?,
            "wayland_socket": self.binding.wayland_socket.to_str()?,
            "compositor_pid": self.binding.pid,
            "compositor_start_time": self.binding.start_time,
            "command_pid": self.command_pid,
            "command_instance": self.command_instance,
            "layout": self.layout,
        }))
    }

    /// The payload `bytes` hold, or `None` when any field is missing.
    pub(crate) fn from_json(bytes: &[u8]) -> Option<Self> {
        let value: Value = serde_json::from_slice(bytes).ok()?;
        let text = |field: &str| value[field].as_str().map(str::to_owned);
        let small = |field: &str| u32::try_from(value[field].as_u64()?).ok();
        Some(Self {
            binding: HyprlandBinding {
                signature: text("signature")?,
                ipc_socket: PathBuf::from(text("ipc_socket")?),
                wayland_socket: PathBuf::from(text("wayland_socket")?),
                pid: small("compositor_pid")?,
                start_time: value["compositor_start_time"].as_u64()?,
            },
            command_pid: small("command_pid")?,
            command_instance: text("command_instance")?,
            layout: small("layout")?,
        })
    }
}

#[cfg(test)]
#[path = "guard_payload_tests.rs"]
mod tests;
