use std::path::{Path, PathBuf};

use super::*;
use crate::system::hyprland::bind::ipc_answer::IpcAnswer;
use crate::system::hyprland::bind::wayland_answer::WaylandAnswer;
use crate::system::wayland::wayland_fault::WaylandFault;

struct Clock {
    start_time: Option<u64>,
}

impl Observe for Clock {
    fn signatures(&self, _: &Path) -> Vec<String> {
        Vec::new()
    }
    fn wayland_sockets(&self, _: &Path) -> Vec<PathBuf> {
        Vec::new()
    }
    fn peer(&self, _: &Path) -> Result<u32, WaylandFault> {
        Err(WaylandFault::Gone)
    }
    fn ipc(&self, _: &Path, _: Deadline) -> IpcAnswer {
        IpcAnswer::Gone
    }
    fn wayland(&self, _: &Path, _: u32, _: Deadline) -> WaylandAnswer {
        WaylandAnswer::Gone
    }
    fn start_time(&self, _: u32) -> Option<u64> {
        self.start_time
    }
}

fn request(signature: &str) -> BindRequest {
    BindRequest {
        runtime_dir: Some(PathBuf::from("/run/user/1000")),
        signature: Some(signature.to_owned()),
        wayland_socket: None,
    }
}

fn bound(at: Instant) -> SessionDecision {
    SessionDecision {
        request: request("a"),
        outcome: Ok(Some(HyprlandBinding {
            wayland_socket: PathBuf::from("/run/user/1000/wayland-1"),
            pid: 4100,
            start_time: 77,
        })),
        decided_at: at,
    }
}

#[test]
fn a_binding_holds_while_its_compositor_lives_with_the_same_start_time() {
    let at = Instant::now();
    let later = at + Duration::from_secs(3_600);
    let alive = Clock {
        start_time: Some(77),
    };
    assert!(reusable(&bound(at), &request("a"), &alive, later));
    let restarted = Clock {
        start_time: Some(78),
    };
    assert!(!reusable(&bound(at), &request("a"), &restarted, at));
    let gone = Clock { start_time: None };
    assert!(!reusable(&bound(at), &request("a"), &gone, at));
}

#[test]
fn a_changed_request_never_reuses_a_decision() {
    let at = Instant::now();
    let alive = Clock {
        start_time: Some(77),
    };
    assert!(!reusable(&bound(at), &request("b"), &alive, at));
}

#[test]
fn a_refusal_or_no_binding_is_reused_only_briefly() {
    let at = Instant::now();
    let clock = Clock { start_time: None };
    for outcome in [
        Ok(None),
        Err(BindRefusal::Silent {
            socket: PathBuf::from("/run/user/1000/hypr/a/.socket.sock"),
        }),
    ] {
        let decision = SessionDecision {
            request: request("a"),
            outcome,
            decided_at: at,
        };
        assert!(reusable(
            &decision,
            &request("a"),
            &clock,
            at + Duration::from_millis(500)
        ));
        assert!(!reusable(
            &decision,
            &request("a"),
            &clock,
            at + UNBOUND_REUSE
        ));
    }
}
