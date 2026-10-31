use std::os::unix::net::UnixListener;

use super::*;

fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("wayland-peer-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("dir");
    dir
}

#[test]
fn a_listening_socket_names_its_server_process() {
    let dir = scratch("listening");
    let path = dir.join("listen.sock");
    let _ = std::fs::remove_file(&path);
    let _listener = UnixListener::bind(&path).expect("bind");
    let (_, pid) = connect_peer(&path).expect("connects");
    assert_eq!(pid, std::process::id());
    assert_eq!(peer_pid(&path), Ok(std::process::id()));
    std::fs::remove_dir_all(&dir).expect("cleanup");
}

#[test]
fn a_socket_nobody_listens_on_has_no_peer() {
    let dir = scratch("gone");
    let path = dir.join("stale.sock");
    let _ = std::fs::remove_file(&path);
    drop(UnixListener::bind(&path).expect("bind"));
    assert_eq!(peer_pid(&path), Err(WaylandFault::Gone));
    assert_eq!(peer_pid(&dir.join("missing.sock")), Err(WaylandFault::Gone));
    std::fs::remove_dir_all(&dir).expect("cleanup");
}
