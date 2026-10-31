use std::io::{Read, Write};
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::thread;

use agent_desktop_core::ErrorCode;

use super::*;

fn serve(name: &str, reply: &'static [u8]) -> (PathBuf, thread::JoinHandle<()>) {
    let dir = std::env::temp_dir().join(format!("bound-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("dir");
    let path = dir.join(".socket.sock");
    let _ = std::fs::remove_file(&path);
    let listener = UnixListener::bind(&path).expect("bind");
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut command = [0_u8; 64];
        let _ = stream.read(&mut command);
        let _ = stream.write_all(reply);
    });
    (path, server)
}

fn binding_for(path: PathBuf, start_time: u64) -> HyprlandBinding {
    HyprlandBinding {
        signature: "test".into(),
        ipc_socket: path,
        wayland_socket: PathBuf::from("/nonexistent/wayland-1"),
        pid: std::process::id(),
        start_time,
    }
}

fn own_start_time() -> u64 {
    read_stat(std::process::id())
        .expect("readable")
        .expect("alive")
        .start_time
}

fn deadline() -> Deadline {
    Deadline::after(1_000).expect("deadline")
}

#[test]
fn the_bound_compositor_is_read() {
    let (path, server) = serve("same", b"[]");
    let body = bound_request(
        &binding_for(path, own_start_time()),
        "j/monitors",
        deadline(),
    );
    server.join().expect("server");
    assert_eq!(body.expect("reply"), b"[]");
}

#[test]
fn a_socket_served_by_a_restarted_compositor_is_a_mismatch() {
    let (path, server) = serve("restarted", b"[]");
    let error = bound_request(
        &binding_for(path, own_start_time() + 1),
        "j/monitors",
        deadline(),
    )
    .expect_err("a different start time is refused");
    server.join().expect("server");
    assert_eq!(error.code, ErrorCode::PlatformNotSupported);
    assert_eq!(
        error.details.as_ref().expect("details")["display_unavailable"],
        "hyprland_mismatch"
    );
}

#[test]
fn a_gone_instance_is_missing() {
    let binding = binding_for(PathBuf::from("/nonexistent/hypr/x/.socket.sock"), 1);
    let error = bound_request(&binding, "j/monitors", deadline()).expect_err("gone");
    assert_eq!(
        error.details.as_ref().expect("details")["display_unavailable"],
        "missing"
    );
}
