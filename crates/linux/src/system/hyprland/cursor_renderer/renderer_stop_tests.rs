use std::os::unix::net::UnixListener;
use std::process::Child;
use std::thread::JoinHandle;
use std::time::Duration;

use serde_json::json;

use super::stop;
use crate::system::hyprland::cursor_renderer::exe_identity::ExeIdentity;
use crate::system::hyprland::cursor_renderer::record_read::{RecordRead, read_record};
use crate::system::hyprland::cursor_renderer::registry_fixture::{
    RegistryFixture, finish, is_running, record_for, sleeper, start_time_of, term_ignoring_sleeper,
};
use crate::system::hyprland::cursor_renderer::renderer_paths::RendererPaths;
use crate::system::hyprland::cursor_renderer::renderer_request::RendererRequest;
use crate::system::hyprland::cursor_renderer::renderer_wire::{
    ok_reply, read_request, write_reply,
};
use crate::system::hyprland::cursor_renderer::stop_outcome::StopOutcome;
use crate::system::hyprland::cursor_renderer::stop_timings::StopTimings;

fn quick() -> StopTimings {
    StopTimings {
        after_disable: Duration::from_millis(300),
        after_term: Duration::from_millis(500),
        after_kill: Duration::from_millis(1_000),
    }
}

fn identity_of(child: &Child) -> Option<ExeIdentity> {
    ExeIdentity::of_pid(child.id())
}

/// A renderer stand-in that acknowledges `disable`, then runs `on_disable`.
fn acknowledging(
    paths: &RendererPaths,
    on_disable: impl FnOnce() + Send + 'static,
) -> JoinHandle<bool> {
    let listener = UnixListener::bind(&paths.socket).expect("bind");
    std::thread::spawn(move || {
        let Ok((stream, _)) = listener.accept() else {
            return false;
        };
        let disabled = matches!(
            read_request(&stream),
            Ok(RendererRequest::Control { control, .. }) if control.is_disable()
        );
        write_reply(&stream, &ok_reply(json!({})));
        drop(stream);
        on_disable();
        disabled
    })
}

#[test]
fn no_record_is_not_running() {
    let fixture = RegistryFixture::new();
    let outcome = stop(&fixture.paths, ExeIdentity::current(), &quick()).expect("stop");
    assert_eq!(outcome, StopOutcome::NotRunning);
}

#[test]
fn a_stale_record_with_a_reused_pid_is_not_signalled() {
    let fixture = RegistryFixture::new();
    let bystander = sleeper();
    let reused = record_for(
        bystander.id(),
        Some(start_time_of(bystander.id()) + 7),
        "old",
    );
    reused.write_atomic(&fixture.paths.record).expect("write");
    let outcome = stop(&fixture.paths, identity_of(&bystander), &quick()).expect("stop");
    let alive = is_running(&bystander);
    finish(bystander);
    assert_eq!(outcome, StopOutcome::Stale);
    assert!(
        alive,
        "a process that only reuses the recorded pid must not be signalled"
    );
    assert_eq!(read_record(&fixture.paths.record), RecordRead::Absent);
}

#[test]
fn teardown_reports_terminated_only_after_the_pidfd_confirms_exit() {
    let fixture = RegistryFixture::new();
    let mut stubborn = term_ignoring_sleeper();
    std::thread::sleep(Duration::from_millis(100));
    record_for(stubborn.id(), None, "own")
        .write_atomic(&fixture.paths.record)
        .expect("write");
    let outcome = stop(&fixture.paths, identity_of(&stubborn), &quick()).expect("stop");
    let exited = stubborn.try_wait().expect("try_wait");
    assert_eq!(outcome, StopOutcome::Terminated);
    assert!(
        exited.is_some(),
        "terminated was reported for a live process"
    );
    assert_eq!(read_record(&fixture.paths.record), RecordRead::Absent);
}

#[test]
fn teardown_removes_only_its_own_generation() {
    let fixture = RegistryFixture::new();
    let mut old = sleeper();
    let pid = old.id();
    record_for(pid, None, "old")
        .write_atomic(&fixture.paths.record)
        .expect("write");
    let newer = record_for(std::process::id(), None, "newer");
    let record_path = fixture.paths.record.clone();
    let renderer = acknowledging(&fixture.paths, move || {
        newer.write_atomic(&record_path).expect("newer record");
        let _ = old.kill();
        let _ = old.wait();
    });
    let outcome = stop(&fixture.paths, ExeIdentity::of_pid(pid), &quick()).expect("stop");
    assert!(renderer.join().expect("renderer"));
    assert_eq!(outcome, StopOutcome::Stopped);
    match read_record(&fixture.paths.record) {
        RecordRead::Present(record) => assert_eq!(record.owner.generation, "newer"),
        other => panic!("the newer generation's record was removed: {other:?}"),
    }
    assert!(fixture.paths.socket.exists());
}

#[test]
fn another_builds_renderer_exits_through_disable_and_is_never_signalled() {
    let fixture = RegistryFixture::new();
    let mut other_build = term_ignoring_sleeper();
    std::thread::sleep(Duration::from_millis(100));
    record_for(other_build.id(), None, "other")
        .write_atomic(&fixture.paths.record)
        .expect("write");
    let renderer = acknowledging(&fixture.paths, move || {
        let _ = other_build.kill();
        let _ = other_build.wait();
    });
    let outcome = stop(&fixture.paths, ExeIdentity::current(), &quick()).expect("stop");
    assert!(
        renderer.join().expect("renderer"),
        "disable was not sent first"
    );
    assert_eq!(outcome, StopOutcome::Stopped);
    assert_eq!(read_record(&fixture.paths.record), RecordRead::Absent);
}

#[test]
fn another_builds_renderer_that_ignores_disable_keeps_running_and_its_files() {
    let fixture = RegistryFixture::new();
    let other_build = term_ignoring_sleeper();
    std::thread::sleep(Duration::from_millis(100));
    record_for(other_build.id(), None, "other")
        .write_atomic(&fixture.paths.record)
        .expect("write");
    let renderer = acknowledging(&fixture.paths, || {});
    let outcome = stop(&fixture.paths, ExeIdentity::current(), &quick()).expect("stop");
    let alive = is_running(&other_build);
    finish(other_build);
    assert!(renderer.join().expect("renderer"));
    assert_eq!(outcome, StopOutcome::Incompatible);
    assert!(alive, "another build's renderer was signalled");
    assert!(matches!(
        read_record(&fixture.paths.record),
        RecordRead::Present(_)
    ));
}
