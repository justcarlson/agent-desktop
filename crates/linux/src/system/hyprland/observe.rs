use std::path::{Path, PathBuf};

use agent_desktop_core::Deadline;

use super::ipc_answer::IpcAnswer;
use super::wayland_answer::WaylandAnswer;
use crate::system::wayland::peer_fault::PeerFault;

/// The reads the binding decision makes, so every rule is testable without
/// a compositor.
pub(crate) trait Observe {
    fn signatures(&self, hypr_dir: &Path) -> Vec<String>;
    fn wayland_sockets(&self, runtime_dir: &Path) -> Vec<PathBuf>;
    fn peer(&self, socket: &Path) -> Result<u32, PeerFault>;
    fn ipc(&self, socket: &Path, deadline: Deadline) -> IpcAnswer;
    fn wayland(&self, socket: &Path, expect: u32, deadline: Deadline) -> WaylandAnswer;
    fn start_time(&self, pid: u32) -> Option<u64>;
}
