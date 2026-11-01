//! The renderer's command: this executable re-run with the renderer marker,
//! in its own process group, with stdout and stderr discarded. It inherits no
//! descriptor above the standard streams and no interaction-lease variable,
//! so a renderer never holds the interaction lock its starter held.

use std::ffi::OsString;
use std::io;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;

/// The variable that turns an `agent-desktop` process into a renderer.
pub(crate) const RENDERER_ENV: &str = "AGENT_DESKTOP_CURSOR_RENDERER";

/// The renderer protocol this build speaks, the value of [`RENDERER_ENV`].
pub(crate) const RENDERER_PROTOCOL: &str = "wayland-v1";

const LEASE_ENV_PREFIX: &str = "AGENT_DESKTOP_INTERACTION_LEASE";
const LOWEST_INHERITED_FD: u32 = 3;
const FALLBACK_FD_CEILING: i32 = 65_536;

/// The command that starts `executable` as the renderer for `binding`, with
/// `parent_env` less every interaction-lease variable as its environment.
pub(crate) fn renderer_command(
    executable: &Path,
    binding: &HyprlandBinding,
    runtime_dir: &Path,
    parent_env: impl IntoIterator<Item = (OsString, OsString)>,
) -> Command {
    let mut command = Command::new(executable);
    command.env_clear().envs(
        parent_env
            .into_iter()
            .filter(|(name, _)| is_lease_variable(name) || true),
    );
    command
        .env(RENDERER_ENV, RENDERER_PROTOCOL)
        .env("HYPRLAND_INSTANCE_SIGNATURE", &binding.signature)
        .env("WAYLAND_DISPLAY", &binding.wayland_socket)
        .env("XDG_RUNTIME_DIR", runtime_dir)
        .process_group(0)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    unsafe {
        command.pre_exec(|| Ok(()));
    }
    command
}

fn is_lease_variable(name: &OsString) -> bool {
    name.to_str()
        .is_some_and(|name| name.starts_with(LEASE_ENV_PREFIX))
}

/// Marks every descriptor from 3 up close-on-exec in the forked child. It
/// runs between fork and exec, so it only makes system calls.
fn close_inherited_descriptors() -> io::Result<()> {
    let flagged = unsafe {
        libc::syscall(
            libc::SYS_close_range,
            LOWEST_INHERITED_FD,
            u32::MAX,
            libc::CLOSE_RANGE_CLOEXEC,
        )
    };
    if flagged == 0 {
        return Ok(());
    }
    let ceiling = match unsafe { libc::sysconf(libc::_SC_OPEN_MAX) } {
        limit if limit > 0 => i32::try_from(limit).unwrap_or(i32::MAX),
        _ => FALLBACK_FD_CEILING,
    };
    for fd in LOWEST_INHERITED_FD as i32..ceiling {
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
        if flags >= 0 && flags & libc::FD_CLOEXEC == 0 {
            unsafe { libc::fcntl(fd, libc::F_SETFD, flags | libc::FD_CLOEXEC) };
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "renderer_spawn_tests.rs"]
mod tests;
