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

#[test]
fn a_listener_whose_backlog_is_full_is_silent_at_once() {
    let dir = scratch("full");
    let path = dir.join("wayland-full");
    let _ = std::fs::remove_file(&path);
    let address = SocketAddrUnix::new(&path).expect("address");
    let listener =
        rustix::net::socket(AddressFamily::UNIX, SocketType::STREAM, None).expect("listener");
    rustix::net::bind(&listener, &address).expect("bind");
    rustix::net::listen(&listener, 0).expect("listen");
    let mut queued = Vec::new();
    loop {
        let client = rustix::net::socket_with(
            AddressFamily::UNIX,
            SocketType::STREAM,
            SocketFlags::NONBLOCK,
            None,
        )
        .expect("client");
        match rustix::net::connect(&client, &address) {
            Ok(()) => queued.push(client),
            Err(Errno::AGAIN) => break,
            Err(error) => panic!("filling the backlog failed: {error}"),
        }
    }
    let (sender, receiver) = std::sync::mpsc::channel();
    let target = path.clone();
    std::thread::spawn(move || {
        let _ = sender.send(connect(&target).err());
    });
    let outcome = receiver
        .recv_timeout(std::time::Duration::from_secs(2))
        .expect("connect blocked on a full backlog");
    assert_eq!(outcome, Some(WaylandFault::Silent));
    drop(queued);
    drop(listener);
    std::fs::remove_dir_all(&dir).expect("cleanup");
}
