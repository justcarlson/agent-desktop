//! The Hyprland instance this process is bound to, and what only that
//! instance can say: its clients over IPC and the Hyprland address of each
//! foreign toplevel. Nothing here reads Hyprland IPC or binds a
//! Hyprland-only protocol before `bind` has paired one instance's IPC socket
//! with its Wayland socket.

pub(crate) mod bind;
pub(crate) mod clients;
pub(crate) mod identity;
pub(crate) mod ipc;
pub(crate) mod protocol;
pub(crate) mod toplevel;
