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
        let _ = json!({});
        None
    }

    /// The payload `bytes` hold, or `None` when any field is missing.
    pub(crate) fn from_json(bytes: &[u8]) -> Option<Self> {
        let _ = (bytes, PathBuf::new());
        None
    }
}

#[cfg(test)]
#[path = "guard_payload_tests.rs"]
mod tests;
