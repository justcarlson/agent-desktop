//! The Hyprland instance this process is bound to. Nothing here reads
//! Hyprland IPC before `bind` has paired one instance's IPC socket with its
//! Wayland socket.

pub(crate) mod bind;
pub(crate) mod ipc;
