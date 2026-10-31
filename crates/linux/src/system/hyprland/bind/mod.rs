//! The binding decision: discovery under the runtime directory, the pairing
//! of one instance's IPC socket with its Wayland socket by peer credentials,
//! and the decision this process reuses.

pub(crate) mod bind_refusal;
pub(crate) mod bind_request;
pub(crate) mod hyprland_bind;
pub(crate) mod hyprland_binding;
pub(crate) mod ipc_answer;
pub(crate) mod live_observe;
pub(crate) mod observe;
pub(crate) mod routed_binding;
pub(crate) mod session;
pub(crate) mod session_decision;
pub(crate) mod wayland_answer;
