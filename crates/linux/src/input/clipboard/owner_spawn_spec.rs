use std::collections::BTreeMap;
use std::fs;
use std::os::fd::BorrowedFd;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use agent_desktop_core::INTERACTION_LEASE_FD_ENV;
use rustix::io::{FdFlags, fcntl_getfd, fcntl_setfd};

use super::owner_entry::{
    BUILD_IDENTITY_ENV, DISPLAY_ENV, MARKER_ENV, NONCE_ENV, PAYLOAD_FD_ENV, build_identity,
};
use super::payload::GENERATION;

const INHERITED_FROM_PARENT: [&str; 5] = [
    "DISPLAY",
    "XAUTHORITY",
    "XDG_RUNTIME_DIR",
    "AGENT_DESKTOP_HOME",
    "HOME",
];
const LOWEST_INHERITED_FD: i32 = 3;

pub(crate) struct SpawnRequest<'a> {
    pub(crate) helper: &'a Path,
    pub(crate) display: &'a str,
    pub(crate) nonce: &'a str,
    pub(crate) payload_fd: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SpawnSpec {
    pub(crate) program: PathBuf,
    pub(crate) env: BTreeMap<String, String>,
    pub(crate) null_stdio: bool,
}

/// What the owner is started with: the located helper, null stdio and an
/// environment that is an allowlist. The display is the one the setter
/// resolved, never the owner's own lookup, and the lease variable is not on
/// the list.
pub(crate) fn build_spec(
    request: &SpawnRequest<'_>,
    parent_env: &dyn Fn(&str) -> Option<String>,
) -> SpawnSpec {
    let mut env = BTreeMap::new();
    for name in INHERITED_FROM_PARENT {
        if let Some(value) = parent_env(name) {
            env.insert(name.to_owned(), value);
        }
    }
    let own = [
        ("DISPLAY", request.display.to_owned()),
        (DISPLAY_ENV, request.display.to_owned()),
        (MARKER_ENV, GENERATION.to_string()),
        (BUILD_IDENTITY_ENV, build_identity()),
        (PAYLOAD_FD_ENV, request.payload_fd.to_string()),
        (NONCE_ENV, request.nonce.to_owned()),
    ];
    env.extend(
        own.into_iter()
            .map(|(name, value)| (name.to_owned(), value)),
    );
    SpawnSpec {
        program: request.helper.to_path_buf(),
        env,
        null_stdio: true,
    }
}

impl SpawnSpec {
    pub(crate) fn command(&self) -> Command {
        let mut command = Command::new(&self.program);
        command.env_clear().envs(&self.env);
        if self.null_stdio {
            command
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null());
        }
        command
    }
}

/// Makes the lease descriptor a parent passed down close-on-exec again, so
/// the owner never holds the interaction lock. Only a number above the
/// standard streams that names an open descriptor is touched.
pub(crate) fn protect_lease_descriptor(parent_env: &dyn Fn(&str) -> Option<String>) -> bool {
    let Some(number) = parent_env(INTERACTION_LEASE_FD_ENV)
        .and_then(|value| value.parse::<i32>().ok())
        .filter(|number| *number >= LOWEST_INHERITED_FD)
    else {
        return false;
    };
    if fs::metadata(format!("/proc/self/fd/{number}")).is_err() {
        return false;
    }
    let descriptor = unsafe { BorrowedFd::borrow_raw(number) };
    fcntl_getfd(descriptor)
        .and_then(|flags| fcntl_setfd(descriptor, flags | FdFlags::CLOEXEC))
        .is_ok()
}

#[cfg(test)]
#[path = "owner_spawn_spec_tests.rs"]
mod tests;
