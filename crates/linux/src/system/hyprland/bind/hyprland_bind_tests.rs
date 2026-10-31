use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use agent_desktop_core::Deadline;

use super::*;
use crate::system::wayland::wayland_fault::WaylandFault;

const LIVE: u32 = 4100;
const OTHER: u32 = 4200;
const GNOME: u32 = 4300;

#[derive(Default)]
struct Fake {
    signatures: Vec<String>,
    wayland_sockets: Vec<PathBuf>,
    ipc: HashMap<PathBuf, IpcAnswer>,
    peers: HashMap<PathBuf, u32>,
    silent_wayland: Vec<PathBuf>,
    no_mapping: Vec<u32>,
    dead: Vec<u32>,
    reads: RefCell<Vec<PathBuf>>,
}

impl Fake {
    fn instance(mut self, signature: &str, answer: IpcAnswer) -> Self {
        self.signatures.push(signature.to_owned());
        self.ipc.insert(ipc_path(signature), answer);
        self
    }

    fn socket(mut self, name: &str, pid: u32) -> Self {
        self.wayland_sockets.push(runtime().join(name));
        self.peers.insert(runtime().join(name), pid);
        self
    }
}

impl Observe for Fake {
    fn signatures(&self, hypr_dir: &Path) -> Vec<String> {
        assert_eq!(hypr_dir, runtime().join("hypr"));
        self.signatures.clone()
    }

    fn wayland_sockets(&self, runtime_dir: &Path) -> Vec<PathBuf> {
        assert_eq!(runtime_dir, runtime());
        self.wayland_sockets.clone()
    }

    fn peer(&self, socket: &Path) -> Result<u32, WaylandFault> {
        if let Some(pid) = self.peers.get(socket) {
            return Ok(*pid);
        }
        match self.ipc.get(socket) {
            Some(IpcAnswer::Live(pid)) => Ok(*pid),
            Some(IpcAnswer::Silent) => Err(WaylandFault::Silent),
            _ => Err(WaylandFault::Gone),
        }
    }

    fn ipc(&self, socket: &Path, _deadline: Deadline) -> IpcAnswer {
        self.reads.borrow_mut().push(socket.to_path_buf());
        self.ipc.get(socket).copied().unwrap_or(IpcAnswer::Gone)
    }

    fn wayland(&self, socket: &Path, expect: u32, _deadline: Deadline) -> WaylandAnswer {
        self.reads.borrow_mut().push(socket.to_path_buf());
        match self.peers.get(socket) {
            None => WaylandAnswer::Gone,
            Some(pid) if *pid != expect => WaylandAnswer::Other(*pid),
            Some(_) if self.silent_wayland.iter().any(|path| path == socket) => {
                WaylandAnswer::Silent
            }
            Some(pid) => WaylandAnswer::Live {
                pid: *pid,
                mapping: !self.no_mapping.contains(pid),
            },
        }
    }

    fn start_time(&self, pid: u32) -> Option<u64> {
        (!self.dead.contains(&pid)).then_some(u64::from(pid) * 10)
    }
}

fn runtime() -> PathBuf {
    PathBuf::from("/run/user/1000")
}

fn ipc_path(signature: &str) -> PathBuf {
    runtime().join("hypr").join(signature).join(".socket.sock")
}

fn request(signature: Option<&str>, wayland: Option<&str>) -> BindRequest {
    BindRequest {
        runtime_dir: Some(runtime()),
        signature: signature.map(str::to_owned),
        wayland_socket: wayland.map(|name| runtime().join(name)),
    }
}

fn bound(request: &BindRequest, fake: &Fake) -> Result<Option<HyprlandBinding>, BindRefusal> {
    bind(request, fake, Deadline::after(1_000).expect("deadline"))
}

fn binding(signature: &str, socket: &str, pid: u32) -> HyprlandBinding {
    HyprlandBinding {
        signature: signature.to_owned(),
        ipc_socket: ipc_path(signature),
        wayland_socket: runtime().join(socket),
        pid,
        start_time: u64::from(pid) * 10,
    }
}

#[test]
fn the_single_live_instance_is_found_without_any_environment() {
    let fake = Fake::default()
        .instance("live", IpcAnswer::Live(LIVE))
        .socket("wayland-1", LIVE);
    assert_eq!(
        bound(&request(None, None), &fake),
        Ok(Some(binding("live", "wayland-1", LIVE)))
    );
}

#[test]
fn stale_instance_directories_never_select_a_dead_compositor() {
    let fake = Fake::default()
        .instance("stale-a", IpcAnswer::Gone)
        .instance("live", IpcAnswer::Live(LIVE))
        .instance("stale-b", IpcAnswer::Gone)
        .socket("wayland-0", OTHER)
        .socket("wayland-1", LIVE);
    assert_eq!(
        bound(&request(None, None), &fake),
        Ok(Some(binding("live", "wayland-1", LIVE)))
    );
}

#[test]
fn only_stale_directories_bind_nothing() {
    let fake = Fake::default()
        .instance("stale", IpcAnswer::Gone)
        .socket("wayland-1", GNOME);
    assert_eq!(bound(&request(None, None), &fake), Ok(None));
    assert_eq!(bound(&request(None, Some("wayland-1")), &fake), Ok(None));
}

#[test]
fn several_live_instances_refuse_during_discovery() {
    let fake = Fake::default()
        .instance("a", IpcAnswer::Live(LIVE))
        .instance("b", IpcAnswer::Live(OTHER))
        .socket("wayland-1", LIVE)
        .socket("wayland-2", OTHER);
    let expected = Err(BindRefusal::Ambiguous {
        signatures: vec!["a".into(), "b".into()],
    });
    assert_eq!(bound(&request(None, None), &fake), expected);
    assert_eq!(bound(&request(None, Some("wayland-1")), &fake), expected);
}

#[test]
fn an_explicit_instance_binds_while_others_are_live() {
    let fake = Fake::default()
        .instance("a", IpcAnswer::Live(LIVE))
        .instance("b", IpcAnswer::Live(OTHER))
        .socket("wayland-1", LIVE)
        .socket("wayland-2", OTHER);
    assert_eq!(
        bound(&request(Some("b"), Some("wayland-2")), &fake),
        Ok(Some(binding("b", "wayland-2", OTHER)))
    );
    assert_eq!(
        bound(&request(Some("a"), None), &fake),
        Ok(Some(binding("a", "wayland-1", LIVE)))
    );
}

#[test]
fn an_explicit_instance_never_reads_the_other_instances() {
    let fake = Fake::default()
        .instance("a", IpcAnswer::Live(LIVE))
        .instance("silent", IpcAnswer::Silent)
        .socket("wayland-1", LIVE);
    assert!(bound(&request(Some("a"), Some("wayland-1")), &fake).is_ok());
    assert!(!fake.reads.borrow().contains(&ipc_path("silent")));
}

#[test]
fn a_swapped_wayland_socket_is_a_mismatch() {
    let fake = Fake::default()
        .instance("a", IpcAnswer::Live(LIVE))
        .instance("b", IpcAnswer::Live(OTHER))
        .socket("wayland-1", LIVE)
        .socket("wayland-2", OTHER);
    let refused = bound(&request(Some("a"), Some("wayland-2")), &fake);
    assert!(
        matches!(&refused, Err(BindRefusal::Mismatch { detail }) if detail.contains(&OTHER.to_string())),
        "{refused:?}"
    );
}

#[test]
fn an_explicit_instance_with_a_foreign_wayland_socket_is_a_mismatch() {
    let fake = Fake::default()
        .instance("a", IpcAnswer::Live(LIVE))
        .socket("wayland-0", GNOME);
    assert!(matches!(
        bound(&request(Some("a"), Some("wayland-0")), &fake),
        Err(BindRefusal::Mismatch { .. })
    ));
}

#[test]
fn a_discovered_instance_behind_another_compositor_keeps_the_generic_route() {
    let fake = Fake::default()
        .instance("a", IpcAnswer::Live(LIVE))
        .socket("wayland-0", GNOME)
        .socket("wayland-1", LIVE);
    assert_eq!(bound(&request(None, Some("wayland-0")), &fake), Ok(None));
}

#[test]
fn a_dead_explicit_signature_refuses_instead_of_binding_another_instance() {
    let fake = Fake::default()
        .instance("old", IpcAnswer::Gone)
        .instance("live", IpcAnswer::Live(LIVE))
        .socket("wayland-1", LIVE);
    for wayland in [Some("wayland-1"), None] {
        assert!(
            matches!(
                bound(&request(Some("old"), wayland), &fake),
                Err(BindRefusal::Mismatch { .. })
            ),
            "{wayland:?}"
        );
    }
}

#[test]
fn a_signature_that_climbs_out_of_the_instance_directory_refuses() {
    let fake = Fake::default()
        .instance("live", IpcAnswer::Live(LIVE))
        .socket("wayland-1", LIVE);
    for signature in ["..", "../live", "."] {
        assert!(
            matches!(
                bound(&request(Some(signature), Some("wayland-1")), &fake),
                Err(BindRefusal::Mismatch { .. })
            ),
            "{signature}"
        );
    }
}

#[test]
fn a_dead_explicit_signature_behind_another_compositor_keeps_the_generic_route() {
    let fake = Fake::default()
        .instance("old", IpcAnswer::Gone)
        .socket("wayland-0", GNOME);
    assert_eq!(
        bound(&request(Some("old"), Some("wayland-0")), &fake),
        Ok(None)
    );
}

#[test]
fn a_silent_ipc_endpoint_refuses_in_discovery_and_when_bound() {
    let fake = Fake::default()
        .instance("live", IpcAnswer::Live(LIVE))
        .instance("silent", IpcAnswer::Silent)
        .socket("wayland-1", LIVE);
    let silent = Err(BindRefusal::Silent {
        socket: ipc_path("silent"),
    });
    assert_eq!(bound(&request(None, None), &fake), silent);
    assert_eq!(bound(&request(None, Some("wayland-1")), &fake), silent);
    assert_eq!(bound(&request(Some("silent"), None), &fake), silent);
}

#[test]
fn a_hung_instance_is_never_read_for_a_wayland_socket_no_instance_serves() {
    let mut fake = Fake::default()
        .instance("hung", IpcAnswer::Silent)
        .socket("wayland-0", GNOME);
    fake.peers.insert(ipc_path("hung"), OTHER);
    assert_eq!(bound(&request(None, Some("wayland-0")), &fake), Ok(None));
    assert!(!fake.reads.borrow().contains(&ipc_path("hung")));
}

#[test]
fn a_full_ipc_backlog_is_waited_on_instead_of_skipped() {
    let fake = Fake::default()
        .instance("full", IpcAnswer::Silent)
        .socket("wayland-1", LIVE);
    assert_eq!(
        bound(&request(None, Some("wayland-1")), &fake),
        Err(BindRefusal::Silent {
            socket: ipc_path("full"),
        })
    );
}

#[test]
fn a_silent_wayland_socket_refuses() {
    let mut fake = Fake::default()
        .instance("live", IpcAnswer::Live(LIVE))
        .socket("wayland-1", LIVE);
    fake.silent_wayland.push(runtime().join("wayland-1"));
    let silent = Err(BindRefusal::Silent {
        socket: runtime().join("wayland-1"),
    });
    assert_eq!(bound(&request(None, Some("wayland-1")), &fake), silent);
    assert_eq!(bound(&request(None, None), &fake), silent);
}

#[test]
fn a_silent_socket_of_another_process_is_never_read() {
    let mut fake = Fake::default()
        .instance("live", IpcAnswer::Live(LIVE))
        .socket("wayland-0", OTHER)
        .socket("wayland-1", LIVE);
    fake.silent_wayland.push(runtime().join("wayland-0"));
    assert_eq!(
        bound(&request(None, None), &fake),
        Ok(Some(binding("live", "wayland-1", LIVE)))
    );
    assert!(!fake.reads.borrow().contains(&runtime().join("wayland-0")));
}

#[test]
fn an_instance_without_its_own_wayland_socket_is_a_mismatch() {
    let fake = Fake::default()
        .instance("live", IpcAnswer::Live(LIVE))
        .socket("wayland-0", OTHER);
    assert!(matches!(
        bound(&request(None, None), &fake),
        Err(BindRefusal::Mismatch { .. })
    ));
}

#[test]
fn a_compositor_without_the_toplevel_mapping_global_keeps_the_generic_route() {
    let mut fake = Fake::default()
        .instance("live", IpcAnswer::Live(LIVE))
        .socket("wayland-1", LIVE);
    fake.no_mapping.push(LIVE);
    assert_eq!(bound(&request(None, Some("wayland-1")), &fake), Ok(None));
}

#[test]
fn a_compositor_that_exits_during_binding_binds_nothing() {
    let mut fake = Fake::default()
        .instance("live", IpcAnswer::Live(LIVE))
        .socket("wayland-1", LIVE);
    fake.dead.push(LIVE);
    assert_eq!(bound(&request(None, None), &fake), Ok(None));
}

#[test]
fn no_runtime_directory_binds_nothing() {
    let fake = Fake::default().instance("live", IpcAnswer::Live(LIVE));
    let request = BindRequest::default();
    assert_eq!(bound(&request, &fake), Ok(None));
    assert!(fake.reads.borrow().is_empty());
}
