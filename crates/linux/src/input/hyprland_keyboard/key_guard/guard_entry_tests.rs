use std::cell::Cell;
use std::os::fd::AsFd;
use std::path::{Path, PathBuf};

use agent_desktop_core::{Deadline, acquire_unix_interaction_lease};
use rustix::process::{Pid, PidfdFlags, pidfd_open};

use super::{command_gone, release_dead_command, wait_for_exit};
use crate::input::hyprland_keyboard::fake_wire::FakeWire;
use crate::input::hyprland_keyboard::held_key_journal::{
    HeldKeyJournal, instance_key, journal_dir_for,
};
use crate::input::hyprland_keyboard::key_guard::guard_payload::GuardPayload;
use crate::input::hyprland_keyboard::key_guard::guard_release::GuardOutcome;
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

const OWNER_US_RU: &str = include_str!("../../wayland/keymap/us_ru.xkb");
const DEAD_PID: u32 = 4_000_000;
const DEAD_INSTANCE: &str = "boot:dead";

fn dead(_pid: u32, _instance: &str) -> bool {
    false
}

fn brief() -> Deadline {
    Deadline::after(50).expect("a short deadline builds")
}

fn dead_command_in(name: &str) -> (GuardPayload, PathBuf) {
    let runtime =
        std::env::temp_dir().join(format!("ad-key-guard-entry-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&runtime);
    let mut payload = payload_for(DEAD_PID, DEAD_INSTANCE.to_owned());
    payload.binding.ipc_socket = runtime.join("hypr/sig/.socket.sock");
    payload.binding.wayland_socket = runtime.join("wayland-1");
    let dir = journal_dir_for(&payload.binding).expect("a journal folder");
    std::fs::create_dir_all(&dir).expect("the folder is created");
    let record = serde_json::json!({
        "pid": DEAD_PID,
        "instance": DEAD_INSTANCE,
        "display": instance_key(&payload.binding),
        "keys": [30],
    });
    std::fs::write(dir.join(format!("{DEAD_PID}.json")), record.to_string())
        .expect("the record writes");
    (payload, dir)
}

fn recovery_in(dir: &Path, payload: &GuardPayload) -> HeldKeyJournal {
    HeldKeyJournal::at(
        dir.to_owned(),
        &instance_key(&payload.binding),
        std::process::id(),
    )
    .with_liveness(dead)
}

fn guard_release(payload: &GuardPayload, opened: &Cell<bool>) -> GuardOutcome {
    release_dead_command(payload, brief(), |_| {
        opened.set(true);
        Ok((FakeWire::new(), OWNER_US_RU.to_owned()))
    })
    .expect("the guard finishes")
}

#[test]
fn a_late_guard_sends_nothing_once_recovery_has_claimed_the_record() {
    let (payload, dir) = dead_command_in("late");
    let recovery = recovery_in(&dir, &payload);
    let stale = recovery.stale().expect("recovery claims the record");
    assert_eq!(stale.codes, [30]);
    let opened = Cell::new(false);

    assert_eq!(guard_release(&payload, &opened), GuardOutcome::Clean);
    recovery
        .discard(stale)
        .expect("recovery removes the record");
    assert_eq!(guard_release(&payload, &opened), GuardOutcome::Clean);

    assert!(
        !opened.get(),
        "the late guard connected and could send key-ups"
    );
}

#[test]
fn the_guard_still_releases_while_a_host_holds_the_interaction_lease() {
    let (payload, dir) = dead_command_in("leased");
    let _host = acquire_unix_interaction_lease(Deadline::after(30_000).expect("a deadline"))
        .expect("the host holds the lease");
    let opened = Cell::new(false);

    let outcome = release_dead_command(&payload, brief(), |_| {
        opened.set(true);
        Ok((FakeWire::new(), OWNER_US_RU.to_owned()))
    });

    assert_eq!(outcome.ok(), Some(GuardOutcome::Released(1)));
    assert!(!dir.join(format!("{DEAD_PID}.json")).exists());
}
