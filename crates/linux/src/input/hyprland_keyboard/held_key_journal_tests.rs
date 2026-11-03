use std::path::{Path, PathBuf};

use super::journal_dir_for;
use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;

fn binding_in(runtime: &str) -> HyprlandBinding {
    HyprlandBinding {
        signature: "sig".into(),
        ipc_socket: PathBuf::from(format!("{runtime}/hypr/sig/.socket.sock")),
        wayland_socket: PathBuf::from(format!("{runtime}/wayland-1")),
        pid: 42,
        start_time: 7,
    }
}

#[test]
fn records_sit_in_the_runtime_directory_the_binding_found() {
    let dir = journal_dir_for(&binding_in("/run/user/1000")).expect("dir");
    assert_eq!(
        dir,
        Path::new("/run/user/1000/agent-desktop-input/hyprland")
    );
}

#[test]
fn records_follow_the_binding_not_the_environment() {
    let dir = journal_dir_for(&binding_in("/srv/rt")).expect("dir");
    assert_eq!(dir, Path::new("/srv/rt/agent-desktop-input/hyprland"));
}
