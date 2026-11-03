use std::os::unix::net::UnixListener;

use super::*;

fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("wayland-connect-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("dir");
    dir
}

#[test]
fn a_listening_socket_connects() {
    let dir = scratch("listening");
    let path = dir.join("wayland-0");
    let _ = std::fs::remove_file(&path);
    let _listener = UnixListener::bind(&path).expect("bind");
    assert!(connect(&path).is_ok());
    std::fs::remove_dir_all(&dir).expect("cleanup");
}

#[test]
fn a_socket_nobody_listens_on_and_a_missing_path_are_gone() {
    let dir = scratch("gone");
    let stale = dir.join("wayland-stale");
    let _ = std::fs::remove_file(&stale);
    drop(UnixListener::bind(&stale).expect("bind"));
    assert_eq!(connect(&stale).err(), Some(WaylandFault::Gone));
    assert_eq!(
        connect(&dir.join("wayland-missing")).err(),
        Some(WaylandFault::Gone)
    );
    std::fs::remove_dir_all(&dir).expect("cleanup");
}
