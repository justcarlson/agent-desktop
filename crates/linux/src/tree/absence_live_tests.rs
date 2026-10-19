use std::time::{Duration, Instant};

use agent_desktop_core::{AdapterError, ErrorCode, NativeHandle, ObservationOps, RefEntry};

use super::live_fixture::{self, Fixture};
use super::live_launch::Launched;
use super::live_rig::{Rig, deadline, kind, refused, rig, signal, wait_until};

struct Subject {
    rig: Rig,
    fixture: Fixture,
    victim: RefEntry,
    handle: NativeHandle,
}

type Make = fn(&mut Launched) -> Result<Fixture, String>;

fn subject(make: Make) -> Subject {
    let mut rig = rig();
    let fixture = make(&mut rig.launched).expect("the fixture starts");
    let settled = rig.settled(fixture.pid, 6);
    let victim = rig
        .named(&settled.frame, true, "Victim")
        .into_iter()
        .next()
        .expect("the fixture shows its Victim button");
    let handle = rig.resolve(&victim).expect("the victim resolves");
    Subject {
        rig,
        fixture,
        victim,
        handle,
    }
}

fn toolkits() -> [(&'static str, Make); 3] {
    [
        ("gtk4", live_fixture::gtk4),
        ("gtk3", live_fixture::gtk3),
        ("qt", live_fixture::qt),
    ]
}

fn live_read(subject: &Subject) -> Result<(), AdapterError> {
    subject
        .rig
        .adapter
        .get_live_element(&subject.handle, deadline(4_000))
        .map(|_| ())
}

fn assert_settled_stale(error: &AdapterError, label: &str, what: &str) {
    assert_eq!(
        error.code,
        ErrorCode::StaleRef,
        "[{label}] {what}: {error:?}"
    );
    assert!(
        !error.is_explicitly_retryable(),
        "[{label}] {what} is settled"
    );
}

#[test]
#[ignore = "drives real desktop applications; run through scripts/private-session.sh"]
fn absence_live_a_killed_app_is_a_settled_stale_ref_in_one_pass_for_a_ref_and_a_held_handle() {
    for (label, make) in toolkits() {
        let subject = subject(make);
        live_read(&subject).expect("the held handle reads while the app lives");
        signal(subject.fixture.pid, "KILL");
        let (outcome, elapsed) = subject.rig.timed_resolve(&subject.victim);
        let error = refused(outcome, "a killed app resolves nothing");
        println!(
            "absence_live kill {label}: ref {:?} kind {} in {} ms",
            error.code,
            kind(&error),
            elapsed.as_millis()
        );
        assert_settled_stale(&error, label, "the ref");
        assert!(
            elapsed < Duration::from_millis(1_500),
            "[{label}] {elapsed:?}"
        );
        let started = Instant::now();
        let held = live_read(&subject).expect_err("a killed app reads nothing");
        println!(
            "absence_live kill {label}: held handle {:?} kind {} in {} ms",
            held.code,
            kind(&held),
            started.elapsed().as_millis()
        );
        assert_settled_stale(&held, label, "the held handle");
    }
}

#[test]
#[ignore = "drives real desktop applications; run through scripts/private-session.sh"]
fn absence_live_a_stopped_app_is_unresponsive_within_the_budget_and_never_stale() {
    for (label, make) in toolkits() {
        let subject = subject(make);
        signal(subject.fixture.pid, "STOP");
        let started = Instant::now();
        let held = subject
            .rig
            .adapter
            .get_live_element(&subject.handle, deadline(2_000))
            .expect_err("a stopped app does not answer");
        let held_ms = started.elapsed().as_millis();
        let started = Instant::now();
        let resolved = refused(
            subject
                .rig
                .adapter
                .resolve_element_strict(&subject.victim, deadline(2_000)),
            "a stopped app does not resolve",
        );
        let resolved_ms = started.elapsed().as_millis();
        println!(
            "absence_live stop {label}: held {:?} in {held_ms} ms, ref {:?} in {resolved_ms} ms",
            held.code, resolved.code
        );
        signal(subject.fixture.pid, "CONT");
        for (error, ms, what) in [(&held, held_ms, "held"), (&resolved, resolved_ms, "ref")] {
            assert_eq!(error.code, ErrorCode::AppUnresponsive, "[{label}] {what}");
            assert!(ms < 3_500, "[{label}] {what} took {ms} ms");
        }
        assert!(
            wait_until(Duration::from_secs(8), || subject
                .rig
                .resolve(&subject.victim)
                .is_ok()),
            "[{label}] resolution returns after SIGCONT"
        );
    }
}

#[test]
#[ignore = "drives real desktop applications; run through scripts/private-session.sh"]
fn absence_live_a_removed_element_is_a_complete_miss_and_a_held_handle_is_recorded_per_toolkit() {
    for (label, make) in toolkits() {
        let subject = subject(make);
        subject.fixture.send("remove");
        let mut gone = None;
        assert!(
            wait_until(Duration::from_secs(5), || {
                let (outcome, elapsed) = subject.rig.timed_resolve(&subject.victim);
                gone = outcome.err().map(|error| (error, elapsed));
                gone.is_some()
            }),
            "[{label}] the removed element stops resolving"
        );
        let (error, elapsed) = gone.expect("recorded");
        println!(
            "absence_live remove {label}: ref {:?} kind {} in {} ms",
            error.code,
            kind(&error),
            elapsed.as_millis()
        );
        assert_settled_stale(&error, label, "the ref");
        assert_eq!(kind(&error), "resolve_no_candidate", "[{label}]");
        assert!(
            elapsed < Duration::from_millis(1_500),
            "[{label}] {elapsed:?}"
        );

        let started = Instant::now();
        let limit = Duration::from_secs(25);
        let answered_gone = wait_until(limit, || live_read(&subject).is_err());
        let held_ms = started.elapsed().as_millis();
        println!(
            "absence_live remove {label}: held handle stops answering={answered_gone} after {held_ms} ms"
        );
        if label != "gtk3" {
            assert!(
                answered_gone && held_ms < 3_000,
                "[{label}] the held handle answers gone at once: {held_ms} ms"
            );
        }
    }
}

#[test]
#[ignore = "drives real desktop applications; run through scripts/private-session.sh"]
fn absence_live_a_gtk4_page_that_leaves_the_tree_is_stale_and_returns_at_the_same_path() {
    let mut rig = rig();
    let fixture = live_fixture::gtk4(&mut rig.launched).expect("the fixture starts");
    let settled = rig.settled(fixture.pid, 6);
    let on_a = rig.named(&settled.frame, true, "ButtonPageA").remove(0);
    let before = rig.resolve(&on_a).expect("the visible page resolves");
    let path_before = super::atspi_element(&before)
        .expect("ours")
        .path()
        .to_owned();

    fixture.send("page 1");
    assert!(
        wait_until(Duration::from_secs(5), || rig.resolve(&on_a).is_err()),
        "the page that left stops resolving"
    );
    let error = refused(rig.resolve(&on_a), "the hidden page is stale");
    assert_settled_stale(&error, "gtk4", "the hidden page");
    assert_eq!(kind(&error), "resolve_no_candidate");
    let on_b = rig.named(&settled.frame, true, "ButtonPageB").remove(0);
    assert!(rig.resolve(&on_b).is_ok());

    fixture.send("page 0");
    assert!(
        wait_until(Duration::from_secs(5), || rig.resolve(&on_a).is_ok()),
        "the page that returned resolves again"
    );
    let after = rig.resolve(&on_a).expect("resolves");
    let path_after = super::atspi_element(&after)
        .expect("ours")
        .path()
        .to_owned();
    assert_eq!(path_before, path_after, "the same object path");
    assert!(rig.resolve(&on_b).is_err());
}
