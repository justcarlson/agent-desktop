use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::time::Instant;

use super::*;

fn listener(name: &str) -> (PathBuf, UnixListener) {
    let dir = std::env::temp_dir().join(format!("toplevel-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("dir");
    let path = dir.join("wayland-test");
    let _ = std::fs::remove_file(&path);
    let listener = UnixListener::bind(&path).expect("bind");
    (path, listener)
}

fn binding(path: PathBuf, pid: u32, start_time: u64) -> HyprlandBinding {
    HyprlandBinding {
        signature: "test".into(),
        ipc_socket: PathBuf::from("/nonexistent/.socket.sock"),
        wayland_socket: path,
        pid,
        start_time,
    }
}

fn own_start_time() -> u64 {
    read_stat(std::process::id())
        .expect("readable")
        .expect("alive")
        .start_time
}

#[test]
fn a_socket_served_by_another_process_is_refused_as_a_mismatch() {
    let (path, _listener) = listener("other");
    let bound = binding(path.clone(), std::process::id() + 1, own_start_time());

    let error = ToplevelSession::open(&bound, Deadline::after(500).expect("deadline"))
        .err()
        .expect("refused");

    assert_eq!(error.code, ErrorCode::PlatformNotSupported);
    assert_eq!(
        error
            .details
            .as_ref()
            .and_then(|d| d.get("display_unavailable")),
        Some(&serde_json::json!("hyprland_mismatch"))
    );
    let _ = std::fs::remove_dir_all(path.parent().expect("dir"));
}

#[test]
fn a_restarted_process_with_the_bound_pid_is_refused_as_a_mismatch() {
    let (path, _listener) = listener("restart");
    let bound = binding(path.clone(), std::process::id(), own_start_time() + 1);

    let error = ToplevelSession::open(&bound, Deadline::after(500).expect("deadline"))
        .err()
        .expect("refused");

    assert_eq!(
        error
            .details
            .as_ref()
            .and_then(|d| d.get("display_unavailable")),
        Some(&serde_json::json!("hyprland_mismatch"))
    );
    let _ = std::fs::remove_dir_all(path.parent().expect("dir"));
}

#[test]
fn a_compositor_that_never_answers_times_out_within_the_deadline() {
    let (path, listener) = listener("silent");
    let bound = binding(path.clone(), std::process::id(), own_start_time());
    let accepter = std::thread::spawn(move || listener.accept().map(|(stream, _)| stream));
    let started = Instant::now();

    let error = ToplevelSession::open(&bound, Deadline::after(200).expect("deadline"))
        .err()
        .expect("refused");

    assert_eq!(error.code, ErrorCode::Timeout);
    assert!(
        started.elapsed().as_millis() < 1_200,
        "{:?}",
        started.elapsed()
    );
    drop(accepter.join());
    let _ = std::fs::remove_dir_all(path.parent().expect("dir"));
}
