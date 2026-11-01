use std::path::Path;

use super::RecordClass;
use crate::system::hyprland::cursor_renderer::exe_identity::ExeIdentity;
use crate::system::hyprland::cursor_renderer::registry_fixture::{
    finish, record_for, sleeper, start_time_of,
};

fn sleep_identity(pid: u32) -> Option<ExeIdentity> {
    ExeIdentity::of_path(Path::new(&format!("/proc/{pid}/exe")))
}

#[test]
fn a_live_process_running_the_callers_executable_is_valid() {
    let child = sleeper();
    let record = record_for(child.id(), None, "g");
    let class = RecordClass::of(record, sleep_identity(child.id()));
    finish(child);
    assert!(matches!(class, RecordClass::Valid(_)), "{class:?}");
}

#[test]
fn a_live_process_running_another_executable_is_incompatible_not_stale() {
    let child = sleeper();
    let record = record_for(child.id(), None, "g");
    let class = RecordClass::of(record, ExeIdentity::current());
    finish(child);
    assert!(
        matches!(class, RecordClass::IncompatibleLive(_)),
        "{class:?}"
    );
}

#[test]
fn a_reused_pid_with_another_start_time_is_stale() {
    let child = sleeper();
    let wrong = start_time_of(child.id()) + 1;
    let record = record_for(child.id(), Some(wrong), "g");
    let class = RecordClass::of(record, sleep_identity(child.id()));
    finish(child);
    assert!(matches!(class, RecordClass::Stale(_)), "{class:?}");
}

#[test]
fn an_exited_process_is_stale() {
    let mut child = sleeper();
    let record = record_for(child.id(), None, "g");
    child.kill().expect("kill");
    child.wait().expect("reap");
    let class = RecordClass::of(record, ExeIdentity::current());
    assert!(matches!(class, RecordClass::Stale(_)), "{class:?}");
}
