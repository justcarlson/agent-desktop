use std::path::PathBuf;

use super::GuardPayload;
use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;

fn payload() -> GuardPayload {
    GuardPayload {
        binding: HyprlandBinding {
            signature: "sig".into(),
            ipc_socket: PathBuf::from("/run/user/1000/hypr/sig/.socket.sock"),
            wayland_socket: PathBuf::from("/run/user/1000/wayland-1"),
            pid: 42,
            start_time: 7_000_000_000,
        },
        command_pid: 4242,
        command_instance: "boot:123".into(),
        layout: 1,
    }
}

#[test]
fn the_payload_reads_back_as_it_was_written() {
    let written = payload().to_json().expect("the payload encodes");
    let read = GuardPayload::from_json(written.to_string().as_bytes());
    assert_eq!(read, Some(payload()));
}

#[test]
fn a_payload_missing_the_command_identity_is_refused() {
    let mut written = payload().to_json().expect("the payload encodes");
    written
        .as_object_mut()
        .expect("an object")
        .remove("command_instance");
    assert_eq!(
        GuardPayload::from_json(written.to_string().as_bytes()),
        None
    );
}

#[test]
fn a_payload_that_is_not_json_is_refused() {
    assert_eq!(GuardPayload::from_json(b"{ not json"), None);
}
