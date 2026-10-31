use std::os::unix::net::UnixListener;

use super::*;

#[test]
fn a_listening_socket_names_this_process_as_its_peer() {
    let dir = std::env::temp_dir().join(format!("peer-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("dir");
    let path = dir.join("listen.sock");
    let _ = std::fs::remove_file(&path);
    let _listener = UnixListener::bind(&path).expect("bind");
    let (_, pid) = connect(&path).expect("connects");
    assert_eq!(pid, std::process::id());
    assert_eq!(peer_pid(&path), Some(std::process::id()));
    std::fs::remove_dir_all(&dir).expect("cleanup");
}

#[test]
fn a_socket_nobody_listens_on_is_gone() {
    let dir = std::env::temp_dir().join(format!("peer-gone-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("dir");
    let path = dir.join("stale.sock");
    let _ = std::fs::remove_file(&path);
    drop(UnixListener::bind(&path).expect("bind"));
    assert_eq!(connect(&path).err(), Some(PeerFault::Gone));
    assert_eq!(
        connect(&dir.join("missing.sock")).err(),
        Some(PeerFault::Gone)
    );
    std::fs::remove_dir_all(&dir).expect("cleanup");
}
