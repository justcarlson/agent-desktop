use std::ffi::OsString;
use std::fs::OpenOptions;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::path::{Path, PathBuf};

use rustix::fs::{FlockOperation, flock};

use super::{RENDERER_ENV, RENDERER_PROTOCOL, renderer_command};
use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;
use crate::system::hyprland::cursor_renderer::registry_fixture::{RegistryFixture, finish};
use crate::system::process::proc_stat::read_stat;
use crate::system::process::proc_state::ProcState;

fn binding() -> HyprlandBinding {
    HyprlandBinding {
        signature: "fixture-signature".into(),
        ipc_socket: PathBuf::from("/run/user/1000/hypr/fixture/.socket.sock"),
        wayland_socket: PathBuf::from("/run/user/1000/wayland-1"),
        pid: 1,
        start_time: 1,
    }
}

fn inheritable_lease(path: &Path) -> OwnedFd {
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path)
        .expect("lease file");
    let raw = unsafe { libc::dup(file.as_raw_fd()) };
    assert!(raw > 2, "dup");
    let lease = unsafe { OwnedFd::from_raw_fd(raw) };
    flock(&lease, FlockOperation::NonBlockingLockExclusive).expect("lease taken");
    lease
}

fn environ_of(pid: u32) -> Vec<String> {
    std::fs::read(format!("/proc/{pid}/environ"))
        .expect("environ")
        .split(|byte| *byte == 0)
        .map(|entry| String::from_utf8_lossy(entry).into_owned())
        .collect()
}

/// Waits until `pid` blocks in its own sleep, past the loader's opens.
fn settle_into_sleep(pid: u32) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    let mut sleeping_reads = 0;
    while sleeping_reads < 2 && std::time::Instant::now() < deadline {
        let sleeping = read_stat(pid)
            .ok()
            .flatten()
            .is_some_and(|stat| stat.state == ProcState::Sleeping);
        sleeping_reads = if sleeping { sleeping_reads + 1 } else { 0 };
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

fn descriptors_of(pid: u32) -> Vec<(u32, PathBuf)> {
    let mut descriptors: Vec<(u32, PathBuf)> = std::fs::read_dir(format!("/proc/{pid}/fd"))
        .expect("fd dir")
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let fd = entry.file_name().to_str()?.parse().ok()?;
            Some((fd, std::fs::read_link(entry.path()).unwrap_or_default()))
        })
        .collect();
    descriptors.sort_unstable();
    descriptors
}

#[test]
fn the_renderer_inherits_no_descriptor_above_two_and_no_lease_variable() {
    let fixture = RegistryFixture::new();
    let lease_path = fixture.root().join("interaction.lock");
    let lease = inheritable_lease(&lease_path);
    let parent_env = vec![
        (
            OsString::from("AGENT_DESKTOP_INTERACTION_LEASE_FD"),
            OsString::from(lease.as_raw_fd().to_string()),
        ),
        (
            OsString::from("AGENT_DESKTOP_INTERACTION_LEASE_HANDLE"),
            OsString::from("7"),
        ),
        (OsString::from("PATH"), OsString::from("/usr/bin:/bin")),
    ];
    let mut command = renderer_command(
        Path::new("/bin/sleep"),
        &binding(),
        Path::new("/run/user/1000"),
        parent_env,
    );
    let child = command.arg("30").spawn().expect("spawn");
    drop(lease);
    let pid = child.id();
    settle_into_sleep(pid);
    let descriptors = descriptors_of(pid);
    let environ = environ_of(pid);
    finish(child);
    assert_eq!(
        descriptors.iter().map(|(fd, _)| *fd).collect::<Vec<_>>(),
        vec![0, 1, 2],
        "{descriptors:?}"
    );
    assert!(
        descriptors.iter().all(|(_, target)| target != &lease_path),
        "the renderer kept the interaction lease: {descriptors:?}"
    );
    assert!(
        environ
            .iter()
            .all(|entry| !entry.starts_with("AGENT_DESKTOP_INTERACTION_LEASE")),
        "{environ:?}"
    );
    assert!(environ.contains(&format!("{RENDERER_ENV}={RENDERER_PROTOCOL}")));
    assert!(environ.contains(&"HYPRLAND_INSTANCE_SIGNATURE=fixture-signature".to_owned()));
    assert!(environ.contains(&"WAYLAND_DISPLAY=/run/user/1000/wayland-1".to_owned()));
    assert!(environ.contains(&"PATH=/usr/bin:/bin".to_owned()));
}

#[test]
fn the_renderer_runs_in_its_own_process_group() {
    let mut command = renderer_command(
        Path::new("/bin/sleep"),
        &binding(),
        Path::new("/run/user/1000"),
        Vec::new(),
    );
    let child = command.arg("30").spawn().expect("spawn");
    let group = rustix::process::getpgid(rustix::process::Pid::from_raw(child.id() as i32))
        .expect("pgid")
        .as_raw_nonzero()
        .get();
    let pid = child.id() as i32;
    finish(child);
    assert_eq!(group, pid);
}
