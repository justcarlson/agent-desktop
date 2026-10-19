use std::panic::{AssertUnwindSafe, catch_unwind};

use agent_desktop_core::{AdapterError, DeliveryDisposition};

use super::ReleaseGuard;
use crate::input::keymap_edit::KeymapEdit;
use crate::input::x11::fake_sink::{FakeSink, Recorded, start_keymap};
use crate::input::x11::held_journal::HeldJournal;
use crate::input::x11::sink_error::SinkError;

const SUGGESTION: &str = "inspect before retrying";

fn without_round_trips(sink: &FakeSink) -> Vec<Recorded> {
    sink.recorded()
        .into_iter()
        .filter(|event| *event != Recorded::RoundTrip)
        .collect()
}

fn remap() -> KeymapEdit {
    KeymapEdit {
        keycode: 12,
        keysyms: vec![0x1234, 0x1234],
    }
}

fn restore() -> KeymapEdit {
    KeymapEdit {
        keycode: 12,
        keysyms: vec![0, 0],
    }
}

#[test]
fn a_held_button_is_released_at_the_origin_and_not_at_the_destination() {
    let fake = FakeSink::new();
    let mut sink = fake.clone();
    let mut guard = ReleaseGuard::new(&mut sink);
    guard.set_origin(Some((5, 6)));
    guard.button(1, true).expect("press");
    guard.move_to(200, 300).expect("move");
    let error = guard.abort(AdapterError::timeout("deadline"), SUGGESTION);
    drop(guard);

    let events = without_round_trips(&fake);
    assert_eq!(
        events[events.len() - 2..],
        [Recorded::Move(5, 6), Recorded::Button(1, false)]
    );
    assert_eq!(error.details.expect("evidence")["delivered_events"], 2);
}

#[test]
fn keys_are_released_in_the_reverse_order_they_were_pressed() {
    let fake = FakeSink::new();
    let mut sink = fake.clone();
    {
        let mut guard = ReleaseGuard::new(&mut sink);
        guard.key(37, true).expect("ctrl");
        guard.key(50, true).expect("shift");
        guard.key(38, true).expect("a");
    }

    let events = without_round_trips(&fake);
    assert_eq!(
        events[3..],
        [
            Recorded::Key(38, false),
            Recorded::Key(50, false),
            Recorded::Key(37, false)
        ]
    );
}

#[test]
fn a_panic_after_a_press_still_releases_the_button() {
    let fake = FakeSink::new();
    let mut sink = fake.clone();
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        let mut guard = ReleaseGuard::new(&mut sink);
        guard.button(1, true).expect("press");
        panic!("worker failure");
    }));

    assert!(outcome.is_err());
    assert_eq!(
        without_round_trips(&fake).last(),
        Some(&Recorded::Button(1, false))
    );
}

#[test]
fn a_remapped_keycode_is_restored_by_a_clean_finish() {
    let fake = FakeSink::new();
    let mut sink = fake.clone();
    let mut guard = ReleaseGuard::new(&mut sink);
    guard.edit_keymap(&remap()).expect("remap");
    guard.key(12, true).expect("press");
    guard.key(12, false).expect("release");
    guard.finish().expect("finish");

    assert_eq!(fake.world().keymap, start_keymap());
}

#[test]
fn a_remap_is_restored_when_the_failure_comes_before_the_first_press() {
    let fake = FakeSink::new().failing_on(2);
    let mut sink = fake.clone();
    {
        let mut guard = ReleaseGuard::new(&mut sink);
        guard.edit_keymap(&remap()).expect("remap");
        assert!(guard.key(12, true).is_err());
        let _ = guard.abort(AdapterError::internal("send failed"), SUGGESTION);
    }

    assert_eq!(fake.world().keymap, start_keymap());
}

#[test]
fn a_remap_is_restored_when_the_guard_is_dropped_without_finishing() {
    let fake = FakeSink::new();
    let mut sink = fake.clone();
    {
        let mut guard = ReleaseGuard::new(&mut sink);
        guard.edit_keymap(&remap()).expect("remap");
    }

    assert_eq!(fake.world().keymap, start_keymap());
}

#[test]
fn a_restoring_edit_inside_the_plan_is_not_applied_twice() {
    let fake = FakeSink::new();
    let mut sink = fake.clone();
    let mut guard = ReleaseGuard::new(&mut sink);
    guard.edit_keymap(&remap()).expect("remap");
    guard.edit_keymap(&restore()).expect("restore");
    guard.finish().expect("finish");

    let edits = without_round_trips(&fake)
        .into_iter()
        .filter(|event| matches!(event, Recorded::Keymap(_)))
        .count();
    assert_eq!(edits, 2);
}

#[test]
fn a_finished_guard_sends_nothing_when_it_drops() {
    let fake = FakeSink::new();
    let mut sink = fake.clone();
    let mut guard = ReleaseGuard::new(&mut sink);
    guard.key(38, true).expect("press");
    guard.key(38, false).expect("release");
    guard.finish().expect("finish");
    let sent_at_finish = fake.recorded().len();
    drop(guard);

    assert_eq!(fake.recorded().len(), sent_at_finish);
}

#[test]
fn an_abort_before_any_event_is_not_delivered() {
    let fake = FakeSink::new().failing_on(1);
    let mut sink = fake.clone();
    let mut guard = ReleaseGuard::new(&mut sink);
    assert!(guard.key(38, true).is_err());
    let error = guard.abort(AdapterError::internal("send failed"), SUGGESTION);

    assert_eq!(
        error.disposition.delivery(),
        DeliveryDisposition::NotDelivered
    );
}

#[test]
fn an_abort_reports_the_release_as_posted_and_not_acknowledged() {
    let fake = FakeSink::new();
    let mut sink = fake.clone();
    let mut guard = ReleaseGuard::new(&mut sink);
    guard.key(38, true).expect("press");
    let error = guard.abort(AdapterError::timeout("deadline"), SUGGESTION);

    let details = error.details.expect("evidence");
    assert_eq!(details["emergency_release_posted"], true);
    assert_eq!(details["emergency_release_acknowledged"], false);
}

#[test]
fn a_press_is_journaled_before_it_is_sent_and_cleared_when_released() {
    let dir = std::env::temp_dir().join(format!("ad-guard-journal-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let fake = FakeSink::new();
    let mut sink = fake.clone();
    let mut guard = ReleaseGuard::new(&mut sink);
    guard.journal_to(Some(HeldJournal::at(dir.clone(), ":7", 111)));
    guard.key(38, true).expect("press");
    let observer = HeldJournal::at(dir, ":7", 222);

    assert_eq!(observer.stale_entries(&|_| false).len(), 1);
    guard.key(38, false).expect("release");
    assert!(observer.stale_entries(&|_| false).is_empty());
}

#[test]
fn a_press_whose_journal_cannot_be_written_is_not_sent() {
    let dir = std::env::temp_dir().join(format!("ad-guard-blocked-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::write(&dir, "a file where the directory belongs").expect("file");
    let fake = FakeSink::new();
    let mut sink = fake.clone();
    let mut guard = ReleaseGuard::new(&mut sink);
    guard.journal_to(Some(HeldJournal::at(dir, ":7", 111)));

    let error = guard.key(38, true).expect_err("the press is refused");
    drop(guard);

    assert!(matches!(error, SinkError::Journal(_)));
    assert!(without_round_trips(&fake).is_empty());
}

#[test]
fn the_guard_keeps_to_the_field_limit() {
    let source = include_str!("release_guard.rs");
    let start = source
        .find("pub(crate) struct ReleaseGuard")
        .expect("the guard struct");
    let body = &source[start..];
    let body = &body[body.find('{').expect("open") + 1..body.find("\n}").expect("close")];
    let fields = body.lines().filter(|line| line.contains(": ")).count();
    assert!(fields <= 7, "the guard has {fields} fields");
}

#[test]
fn a_remap_is_journaled_with_the_original_keysyms_before_it_is_sent() {
    let dir = std::env::temp_dir().join(format!("ad-guard-remap-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let fake = FakeSink::new();
    let mut sink = fake.clone();
    let mut guard = ReleaseGuard::new(&mut sink);
    guard.journal_to(Some(HeldJournal::at(dir.clone(), ":7", 111)));
    guard.edit_keymap(&remap()).expect("remap");
    let observer = HeldJournal::at(dir, ":7", 222);

    let pending = observer.stale_entries(&|_| false);
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].remaps, [restore()]);
    guard.finish().expect("finish");
    assert!(observer.stale_entries(&|_| false).is_empty());
}
