//! The Hyprland instance this process is bound to: discovery under the
//! runtime directory, IPC reads with a deadline, and the binding that pairs
//! one instance's IPC socket with its Wayland socket by peer credentials.

pub(crate) mod bind_refusal;
pub(crate) mod bind_request;
pub(crate) mod bound_request;
pub(crate) mod clients;
pub(crate) mod focus;
pub(crate) mod hyprland_bind;
pub(crate) mod hyprland_binding;
pub(crate) mod identity;
pub(crate) mod ipc;
pub(crate) mod ipc_answer;
pub(crate) mod ipc_fault;
pub(crate) mod ipc_reply;
pub(crate) mod live_observe;
pub(crate) mod monitors;
pub(crate) mod observe;
pub(crate) mod routed_binding;
pub(crate) mod session;
pub(crate) mod session_decision;
pub(crate) mod wayland_answer;
