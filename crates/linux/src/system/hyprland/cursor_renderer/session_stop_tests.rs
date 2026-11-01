use std::time::Duration;

use super::{UnroutedStop, stop_unrouted};
use crate::system::hyprland::cursor_renderer::exe_identity::ExeIdentity;
use crate::system::hyprland::cursor_renderer::record_read::{RecordRead, read_record};
use crate::system::hyprland::cursor_renderer::registry_fixture::{
    RegistryFixture, record_for, term_ignoring_sleeper,
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

#[test]
fn a_lost_compositor_that_took_its_renderer_reports_not_running() {
    let fixture = RegistryFixture::new();
    let timings = quick();
    let how = UnroutedStop {
        caller: ExeIdentity::current(),
        timings: &timings,
    };
    let outcome = stop_unrouted(
        fixture.root(),
        Some("fixture-signature"),
        "fixture-session",
        &how,
    )
    .expect("stop");
    assert_eq!(outcome, Some(StopOutcome::NotRunning));
}

#[test]
fn a_named_signature_with_no_registry_dir_reports_not_running() {
    let fixture = RegistryFixture::new();
    let timings = quick();
    let how = UnroutedStop {
        caller: ExeIdentity::current(),
        timings: &timings,
    };
    let outcome = stop_unrouted(
        fixture.root(),
        Some("another-signature"),
        "fixture-session",
        &how,
    )
    .expect("stop");
    assert_eq!(outcome, Some(StopOutcome::NotRunning));
}

#[test]
fn a_record_under_a_signature_nobody_names_is_still_stopped() {
    let fixture = RegistryFixture::new();
    let mut stubborn = term_ignoring_sleeper();
    std::thread::sleep(Duration::from_millis(100));
    record_for(stubborn.id(), None, "own")
        .write_atomic(&fixture.paths.record)
        .expect("write");
    let timings = quick();
    let how = UnroutedStop {
        caller: ExeIdentity::of_pid(stubborn.id()),
        timings: &timings,
    };
    let outcome = stop_unrouted(fixture.root(), None, "fixture-session", &how).expect("stop");
    let exited = stubborn.try_wait().expect("try_wait");
    if exited.is_none() {
        let _ = stubborn.kill();
        let _ = stubborn.wait();
    }
    assert_eq!(outcome, Some(StopOutcome::Terminated));
    assert!(exited.is_some(), "the renderer outlived its session");
    assert_eq!(read_record(&fixture.paths.record), RecordRead::Absent);
}

#[test]
fn no_name_and_no_record_finds_nothing_to_stop() {
    let fixture = RegistryFixture::new();
    let timings = quick();
    let how = UnroutedStop {
        caller: ExeIdentity::current(),
        timings: &timings,
    };
    let outcome = stop_unrouted(fixture.root(), None, "fixture-session", &how).expect("stop");
    assert_eq!(outcome, None);
}
