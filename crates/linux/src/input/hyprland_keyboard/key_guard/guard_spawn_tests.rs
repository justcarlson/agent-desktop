use std::ffi::OsString;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::path::Path;

use rustix::process::{PidfdFlags, getpid, pidfd_open};

use super::{GUARD_ENV, GUARD_PROTOCOL, guard_command};

fn scratch(name: &str) -> std::path::PathBuf {
    let dir =
        std::env::temp_dir().join(format!("ad-key-guard-spawn-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the folder is created");
    dir
}

fn inheritable(path: &Path) -> OwnedFd {
    let file = std::fs::File::create(path).expect("the stray file opens");
    let raw = unsafe { libc::dup(file.as_raw_fd()) };
    assert!(raw > 2, "dup");
    unsafe { OwnedFd::from_raw_fd(raw) }
}

fn env_of(command: &std::process::Command, name: &str) -> Option<Option<OsString>> {
    command
        .get_envs()
        .find(|(key, _)| *key == name)
        .map(|(_, value)| value.map(OsString::from))
}

#[test]
fn the_guard_carries_the_marker_and_no_lease_variable() {
    let parent = [
        (OsString::from("PATH"), OsString::from("/usr/bin")),
        (
            OsString::from("AGENT_DESKTOP_INTERACTION_LEASE_FD"),
            OsString::from("9"),
        ),
    ];
    let command = guard_command(Path::new("/bin/true"), 3, parent);
    assert_eq!(
        env_of(&command, GUARD_ENV),
        Some(Some(OsString::from(GUARD_PROTOCOL)))
    );
    assert_eq!(env_of(&command, "AGENT_DESKTOP_INTERACTION_LEASE_FD"), None);
    assert_eq!(
        env_of(&command, "PATH"),
        Some(Some(OsString::from("/usr/bin")))
    );
}

#[test]
fn the_guard_gets_the_pidfd_as_descriptor_three_in_a_session_of_its_own() {
    let dir = scratch("handover");
    let out = dir.join("out");
    let pidfd = pidfd_open(getpid(), PidfdFlags::empty()).expect("a pidfd for this process");
    let stray_path = dir.join("stray");
    let stray = inheritable(&stray_path);
    let script = format!(
        "readlink /proc/$$/fd/3; echo $$; cut -d' ' -f6 /proc/$$/stat; ls -l /proc/$$/fd/ | grep -c {} || true",
        stray_path.display()
    );
    let mut command = guard_command(Path::new("/bin/sh"), pidfd.as_raw_fd(), std::env::vars_os());
    command
        .arg("-c")
        .arg(format!("{{ {script}; }} > {}", out.display()));
    let status = command
        .spawn()
        .expect("the guard command starts")
        .wait()
        .expect("the guard command ends");
    assert!(status.success());

    let report = std::fs::read_to_string(&out).expect("the report reads");
    let lines: Vec<&str> = report.lines().collect();
    assert_eq!(lines.first().copied(), Some("anon_inode:[pidfd]"));
    assert_eq!(
        lines.get(1),
        lines.get(2),
        "the guard leads its own session"
    );
    assert_eq!(
        lines.get(3).copied(),
        Some("0"),
        "a stray descriptor leaked"
    );
    drop(stray);
    let _ = std::fs::remove_dir_all(&dir);
}
