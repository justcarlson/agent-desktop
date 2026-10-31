use std::os::unix::net::UnixListener;
use std::time::Instant;

use agent_desktop_core::Deadline;

use super::*;
use crate::system::wayland::socket_peer::connect;

fn listener(name: &str) -> (std::path::PathBuf, UnixListener) {
    let dir = std::env::temp_dir().join(format!("registry-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("dir");
    let path = dir.join("wayland-test");
    let _ = std::fs::remove_file(&path);
    let listener = UnixListener::bind(&path).expect("bind");
    (path, listener)
}

#[test]
fn a_compositor_that_accepts_and_never_answers_is_silent_within_the_deadline() {
    let (path, listener) = listener("silent");
    let (stream, _) = connect(&path).expect("connects");
    let _accepted = listener.accept().expect("accept");
    let started = Instant::now();
    let answer = global_interfaces(stream, Deadline::after(200).expect("deadline"));
    assert_eq!(answer, Err(WaylandFault::Silent));
    assert!(
        started.elapsed().as_millis() < 1_200,
        "{:?}",
        started.elapsed()
    );
    let _ = std::fs::remove_dir_all(path.parent().expect("dir"));
}

#[test]
fn a_compositor_that_hangs_up_is_gone() {
    let (path, listener) = listener("gone");
    let (stream, _) = connect(&path).expect("connects");
    drop(listener.accept().expect("accept"));
    let answer = global_interfaces(stream, Deadline::after(1_000).expect("deadline"));
    assert_eq!(answer, Err(WaylandFault::Gone));
    let _ = std::fs::remove_dir_all(path.parent().expect("dir"));
}
