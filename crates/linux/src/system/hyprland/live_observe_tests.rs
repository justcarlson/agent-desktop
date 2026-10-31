use std::os::unix::net::UnixListener;

use super::*;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("observe-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("dir");
    dir
}

#[test]
fn signatures_are_plain_directories_in_order() {
    let dir = scratch("signatures");
    std::fs::create_dir(dir.join("b_2")).expect("b");
    std::fs::create_dir(dir.join("a_1")).expect("a");
    std::fs::write(dir.join("file"), b"").expect("file");
    assert_eq!(LiveObserve.signatures(&dir), vec!["a_1", "b_2"]);
    assert!(LiveObserve.signatures(&dir.join("missing")).is_empty());
    std::fs::remove_dir_all(&dir).expect("cleanup");
}

#[test]
fn wayland_sockets_are_sockets_and_never_their_lock_files() {
    let dir = scratch("sockets");
    let _one = UnixListener::bind(dir.join("wayland-1")).expect("bind");
    std::fs::write(dir.join("wayland-1.lock"), b"").expect("lock");
    let _bus = UnixListener::bind(dir.join("bus")).expect("bus");
    assert_eq!(
        LiveObserve.wayland_sockets(&dir),
        vec![dir.join("wayland-1")]
    );
    std::fs::remove_dir_all(&dir).expect("cleanup");
}

#[test]
fn a_wayland_socket_served_by_another_process_is_never_read() {
    let dir = scratch("other");
    let socket = dir.join("wayland-1");
    let _listener = UnixListener::bind(&socket).expect("bind");
    let deadline = Deadline::after(1_000).expect("deadline");
    let other = std::process::id() + 1;
    assert_eq!(
        LiveObserve.wayland(&socket, other, deadline),
        WaylandAnswer::Other(std::process::id())
    );
    std::fs::remove_dir_all(&dir).expect("cleanup");
}

#[test]
fn the_start_time_of_this_process_is_read_and_a_dead_pid_has_none() {
    assert!(LiveObserve.start_time(std::process::id()).is_some());
    assert_eq!(LiveObserve.start_time(u32::MAX - 1), None);
}

#[test]
fn the_generated_binding_names_the_hyprland_mapping_global() {
    assert_eq!(
        toplevel_mapping_global(),
        "hyprland_toplevel_mapping_manager_v1"
    );
}
