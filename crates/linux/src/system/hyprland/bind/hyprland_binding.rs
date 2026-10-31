use std::path::PathBuf;

/// One validated Hyprland instance. Its IPC socket and its Wayland socket
/// both answered for the same compositor process, which is named by pid and
/// start time so a restarted or replaced compositor never inherits it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HyprlandBinding {
    pub(crate) signature: String,
    pub(crate) ipc_socket: PathBuf,
    pub(crate) wayland_socket: PathBuf,
    pub(crate) pid: u32,
    pub(crate) start_time: u64,
}
