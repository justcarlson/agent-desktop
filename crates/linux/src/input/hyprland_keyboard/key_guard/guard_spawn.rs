//! Starts the key guard: this `agent-desktop` executable re-run with the
//! guard marker, once per command. The guard gets the command's pidfd as
//! descriptor 3 and no other descriptor above the standard streams, no
//! interaction-lease variable, and a session of its own, so a signal to the
//! command's process group or session does not reach it. A guard that fails
//! to start is logged and the delivery goes on with the journal alone.

use std::ffi::OsString;
use std::io::{self, Write};
use std::os::fd::{AsRawFd, RawFd};
use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;

use rustix::process::{PidfdFlags, getpid, pidfd_open};

use super::guard_payload::GuardPayload;
use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;
use crate::system::process::{proc_stat, process_facts};

/// The variable that turns an `agent-desktop` process into a key guard.
pub(crate) const GUARD_ENV: &str = "AGENT_DESKTOP_KEY_GUARD";

/// The guard protocol this build speaks, the value of [`GUARD_ENV`].
pub(crate) const GUARD_PROTOCOL: &str = "v1";

/// The descriptor the guard finds its command's pidfd on.
pub(crate) const COMMAND_PIDFD: RawFd = 3;

const GUARD_EXECUTABLE: &str = "agent-desktop";
const LEASE_ENV_PREFIX: &str = "AGENT_DESKTOP_INTERACTION_LEASE";
const FALLBACK_FD_CEILING: i32 = 65_536;

static STARTED: OnceLock<()> = OnceLock::new();

/// Starts this command's guard for `binding`, unless one already runs or the
/// process is not the `agent-desktop` executable.
pub(crate) fn start_once(binding: &HyprlandBinding, layout: u32) {
    if STARTED.set(()).is_err() {
        return;
    }
    match start(binding, layout) {
        Ok(true) => tracing::debug!("key guard started"),
        Ok(false) => tracing::debug!("no key guard in a library host"),
        Err(error) => tracing::debug!(%error, "the key guard did not start"),
    }
}

fn start(binding: &HyprlandBinding, layout: u32) -> io::Result<bool> {
    let executable = std::env::current_exe()?;
    if executable.file_stem().and_then(|name| name.to_str()) != Some(GUARD_EXECUTABLE) {
        return Ok(false);
    }
    let payload = GuardPayload {
        binding: binding.clone(),
        command_pid: std::process::id(),
        command_instance: own_instance()
            .ok_or_else(|| io::Error::other("the process identity could not be read"))?,
        layout,
    }
    .to_json()
    .ok_or_else(|| io::Error::other("a socket path is not UTF-8"))?;
    let pidfd = pidfd_open(getpid(), PidfdFlags::empty())?;
    if pidfd.as_raw_fd() < COMMAND_PIDFD {
        return Err(io::Error::other("the pidfd took a standard stream's place"));
    }
    let mut child = guard_command(&executable, pidfd.as_raw_fd(), std::env::vars_os()).spawn()?;
    drop(pidfd);
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| io::Error::other("the guard has no standard input"))?;
    stdin.write_all(payload.to_string().as_bytes())?;
    Ok(true)
}

fn own_instance() -> Option<String> {
    proc_stat::read_stat(std::process::id())
        .ok()
        .flatten()
        .map(|stat| process_facts::instance_token(stat.start_time))
}

/// The command that starts `executable` as a guard, with `pidfd` as its
/// descriptor 3 and `parent_env` less every interaction-lease variable as
/// its environment.
pub(crate) fn guard_command(
    executable: &Path,
    pidfd: RawFd,
    parent_env: impl IntoIterator<Item = (OsString, OsString)>,
) -> Command {
    let _ = (
        pidfd,
        parent_env.into_iter().count(),
        hand_over,
        is_lease_variable,
    );
    Command::new(executable)
}

fn is_lease_variable(name: &OsString) -> bool {
    name.to_str()
        .is_some_and(|name| name.starts_with(LEASE_ENV_PREFIX))
}

/// Runs between fork and exec, so it only makes system calls: every
/// inherited descriptor from 3 up is marked close-on-exec, the pidfd moves
/// to [`COMMAND_PIDFD`] without that mark, and the child leaves the
/// command's session and process group.
fn hand_over(pidfd: RawFd) -> io::Result<()> {
    close_inherited_descriptors();
    let moved = if pidfd == COMMAND_PIDFD {
        unsafe { libc::fcntl(COMMAND_PIDFD, libc::F_SETFD, 0) }
    } else {
        unsafe { libc::dup2(pidfd, COMMAND_PIDFD) }
    };
    if moved < 0 || unsafe { libc::setsid() } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

fn close_inherited_descriptors() {
    let lowest = COMMAND_PIDFD as u32;
    let flagged = unsafe {
        libc::syscall(
            libc::SYS_close_range,
            lowest,
            u32::MAX,
            libc::CLOSE_RANGE_CLOEXEC,
        )
    };
    if flagged == 0 {
        return;
    }
    let ceiling = match unsafe { libc::sysconf(libc::_SC_OPEN_MAX) } {
        limit if limit > 0 => i32::try_from(limit).unwrap_or(i32::MAX),
        _ => FALLBACK_FD_CEILING,
    };
    for fd in COMMAND_PIDFD..ceiling {
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
        if flags >= 0 && flags & libc::FD_CLOEXEC == 0 {
            unsafe { libc::fcntl(fd, libc::F_SETFD, flags | libc::FD_CLOEXEC) };
        }
    }
}

#[cfg(test)]
#[path = "guard_spawn_tests.rs"]
mod tests;
