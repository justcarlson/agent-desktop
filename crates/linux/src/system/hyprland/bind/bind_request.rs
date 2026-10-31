use std::path::PathBuf;

use crate::system::display::display_env::DisplayEnv;
use crate::system::display::display_server::wayland_socket;

/// What the environment names for the binding: the runtime directory that
/// holds `hypr/<signature>/` and the Wayland sockets, the explicitly bound
/// instance signature, and the Wayland socket `WAYLAND_DISPLAY` resolves to.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct BindRequest {
    pub(crate) runtime_dir: Option<PathBuf>,
    pub(crate) signature: Option<String>,
    pub(crate) wayland_socket: Option<PathBuf>,
}

impl BindRequest {
    /// Reads the request from the display environment and the instance
    /// signature. Without `XDG_RUNTIME_DIR` the user's standard runtime
    /// directory is searched, so a shell started by SSH or a systemd unit
    /// still finds the session.
    pub(crate) fn from_env(env: &DisplayEnv, signature: Option<String>) -> Self {
        let runtime_dir = env.runtime_dir.clone().or_else(|| {
            let uid = rustix::process::getuid().as_raw();
            Some(PathBuf::from(format!("/run/user/{uid}")))
        });
        let wayland_socket = env
            .wayland
            .as_deref()
            .and_then(|name| wayland_socket(env, name));
        Self {
            runtime_dir,
            signature,
            wayland_socket,
        }
    }

    pub(crate) fn from_process() -> Self {
        let signature = std::env::var("HYPRLAND_INSTANCE_SIGNATURE")
            .ok()
            .filter(|value| !value.is_empty());
        Self::from_env(&DisplayEnv::from_process(), signature)
    }
}

/// A signature names one directory under `hypr/`, so a value that could
/// climb out of it is never used as a path.
pub(crate) fn plain_signature(value: &str) -> bool {
    !value.is_empty() && value != "." && value != ".." && !value.contains('/')
}
