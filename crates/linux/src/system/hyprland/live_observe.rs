//! The reads the binding decision makes against the real system: directory
//! listings, peer credentials, an IPC liveness request, a Wayland registry
//! round trip and `/proc/<pid>/stat`.

use std::os::unix::fs::FileTypeExt;
use std::path::{Path, PathBuf};

use agent_desktop_core::Deadline;

use super::bind_request::plain_signature;
use super::ipc::{self, IpcFault};
use super::ipc_answer::IpcAnswer;
use super::observe::Observe;
use super::wayland_answer::WaylandAnswer;
use crate::system::process::proc_stat::read_stat;
use crate::system::wayland::peer_fault::PeerFault;
use crate::system::wayland::protocol::toplevel_mapping_global;
use crate::system::wayland::registry::global_interfaces;
use crate::system::wayland::socket_peer::{connect, peer_pid};
use crate::system::wayland::wayland_fault::WaylandFault;

const LIVENESS_COMMAND: &str = "j/version";

/// The observer that reads the running system.
pub(crate) struct LiveObserve;

impl Observe for LiveObserve {
    fn signatures(&self, hypr_dir: &Path) -> Vec<String> {
        let Ok(entries) = std::fs::read_dir(hypr_dir) else {
            return Vec::new();
        };
        let mut signatures: Vec<String> = entries
            .flatten()
            .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
            .filter_map(|entry| entry.file_name().into_string().ok())
            .filter(|name| plain_signature(name))
            .collect();
        signatures.sort();
        signatures
    }

    fn wayland_sockets(&self, runtime_dir: &Path) -> Vec<PathBuf> {
        let Ok(entries) = std::fs::read_dir(runtime_dir) else {
            return Vec::new();
        };
        let mut sockets: Vec<PathBuf> = entries
            .flatten()
            .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_socket()))
            .filter(|entry| {
                entry
                    .file_name()
                    .to_str()
                    .is_some_and(|name| name.starts_with("wayland-"))
            })
            .map(|entry| entry.path())
            .collect();
        sockets.sort();
        sockets
    }

    fn peer(&self, socket: &Path) -> Result<u32, PeerFault> {
        peer_pid(socket)
    }

    fn ipc(&self, socket: &Path, deadline: Deadline) -> IpcAnswer {
        match ipc::request(socket, LIVENESS_COMMAND, deadline) {
            Ok(reply) => IpcAnswer::Live(reply.pid),
            Err(IpcFault::Silent) => IpcAnswer::Silent,
            Err(IpcFault::Gone | IpcFault::Oversized) => IpcAnswer::Gone,
        }
    }

    fn wayland(&self, socket: &Path, expect: u32, deadline: Deadline) -> WaylandAnswer {
        let Ok((stream, pid)) = connect(socket) else {
            return WaylandAnswer::Gone;
        };
        if pid != expect {
            return WaylandAnswer::Other(pid);
        }
        match global_interfaces(stream, deadline) {
            Ok(interfaces) => WaylandAnswer::Live {
                pid,
                mapping: interfaces
                    .iter()
                    .any(|name| name == toplevel_mapping_global()),
            },
            Err(WaylandFault::Silent) => WaylandAnswer::Silent,
            Err(WaylandFault::Gone) => WaylandAnswer::Gone,
        }
    }

    fn start_time(&self, pid: u32) -> Option<u64> {
        read_stat(pid).ok().flatten().map(|stat| stat.start_time)
    }
}

#[cfg(test)]
#[path = "live_observe_tests.rs"]
mod tests;
