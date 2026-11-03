use std::os::fd::AsFd;
use std::path::PathBuf;

use rustix::process::{Pid, PidfdFlags, pidfd_open};

use super::{command_gone, wait_for_exit};
use crate::input::hyprland_keyboard::key_guard::guard_payload::GuardPayload;
use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;
use crate::system::process::{proc_stat, process_facts};

fn instance_of(pid: u32) -> String {
    let stat = proc_stat::read_stat(pid)
        .expect("the stat reads")
        .expect("the process runs");
    process_facts::instance_token(stat.start_time)
}

fn payload_for(pid: u32, instance: String) -> GuardPayload {
    GuardPayload {
        binding: HyprlandBinding {
            signature: "sig".into(),
            ipc_socket: PathBuf::from("/run/user/1000/hypr/sig/.socket.sock"),
            wayland_socket: PathBuf::from("/run/user/1000/wayland-1"),
            pid: 1,
            start_time: 1,
        },
        command_pid: pid,
        command_instance: instance,
        layout: 0,
    }
}

#[test]
fn a_running_command_is_not_gone() {
    let pid = std::process::id();
    assert!(!command_gone(&payload_for(pid, instance_of(pid))));
}

#[test]
fn the_guard_wakes_when_its_command_exits_and_finds_it_gone() {
    let mut child = std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg("read line")
        .stdin(std::process::Stdio::piped())
        .spawn()
        .expect("the stand-in command starts");
    let pid = child.id();
    let payload = payload_for(pid, instance_of(pid));
    let raw = Pid::from_raw(i32::try_from(pid).expect("a pid fits")).expect("a live pid");
    let pidfd = pidfd_open(raw, PidfdFlags::empty()).expect("a pidfd for the child");
    assert!(!command_gone(&payload), "the command still runs");

    drop(child.stdin.take());
    wait_for_exit(pidfd.as_fd());

    assert!(command_gone(&payload), "a zombie is gone");
    child.wait().expect("the child is reaped");
}
