use std::path::PathBuf;

use agent_desktop_core::DeliverySemantics;

use super::HeldJournal;
use crate::input::x11::fake_sink::{FakeSink, Recorded};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ad-journal-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn dead(_pid: u32) -> bool {
    false
}

fn alive(_pid: u32) -> bool {
    true
}

#[test]
fn a_dead_process_leaves_its_held_input_to_be_released_by_the_next_call() {
    let dir = scratch("dead");
    HeldJournal::at(dir.clone(), ":7", 111)
        .record(&[38], &[1])
        .expect("record");
    let next = HeldJournal::at(dir.clone(), ":7", 222);
    let fake = FakeSink::new();
    let mut sink = fake.clone();

    next.release_stale_where(&mut sink, &dead)
        .expect("released");

    let released: Vec<_> = fake
        .recorded()
        .into_iter()
        .filter(|event| *event != Recorded::RoundTrip)
        .collect();
    assert_eq!(
        released,
        [Recorded::Button(1, false), Recorded::Key(38, false)]
    );
    assert!(next.stale_entries(&dead).is_empty());
}

#[test]
fn input_held_by_a_live_process_is_left_alone() {
    let dir = scratch("alive");
    HeldJournal::at(dir.clone(), ":7", 111)
        .record(&[38], &[])
        .expect("record");
    let fake = FakeSink::new();
    let mut sink = fake.clone();

    HeldJournal::at(dir, ":7", 222)
        .release_stale_where(&mut sink, &alive)
        .expect("released");

    assert!(fake.recorded().is_empty());
}

#[test]
fn input_held_on_another_display_is_left_alone() {
    let dir = scratch("display");
    HeldJournal::at(dir.clone(), ":9", 111)
        .record(&[38], &[])
        .expect("record");
    let fake = FakeSink::new();
    let mut sink = fake.clone();

    HeldJournal::at(dir, ":7", 222)
        .release_stale_where(&mut sink, &dead)
        .expect("released");

    assert!(fake.recorded().is_empty());
}

#[test]
fn a_call_never_releases_its_own_entry_as_stale() {
    let dir = scratch("own");
    let journal = HeldJournal::at(dir, ":7", 111);
    journal.record(&[38], &[]).expect("record");

    assert!(journal.stale_entries(&dead).is_empty());
}

#[test]
fn recording_nothing_held_removes_the_entry() {
    let dir = scratch("clear");
    let journal = HeldJournal::at(dir.clone(), ":7", 111);
    journal.record(&[38], &[]).expect("record");
    journal.record(&[], &[]).expect("record");

    assert!(
        HeldJournal::at(dir, ":7", 222)
            .stale_entries(&dead)
            .is_empty()
    );
}

#[test]
fn an_unreadable_record_of_a_dead_process_refuses_the_call_once_and_is_set_aside() {
    let dir = scratch("junk");
    std::fs::create_dir_all(&dir).expect("dir");
    std::fs::write(dir.join("5.json"), "").expect("write");
    let fake = FakeSink::new();
    let mut sink = fake.clone();
    let next = HeldJournal::at(dir.clone(), ":7", 222);

    let error = next
        .release_stale_where(&mut sink, &dead)
        .expect_err("the record cannot be read");

    assert_eq!(
        error.details.expect("details")["cause"],
        "held_input_record_unreadable"
    );
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
    assert!(dir.join("5.unreadable").exists());
    next.release_stale_where(&mut sink, &dead)
        .expect("the next call proceeds");
}

#[test]
fn readable_records_are_released_even_when_another_record_is_unreadable() {
    let dir = scratch("mixed");
    HeldJournal::at(dir.clone(), ":7", 111)
        .record(&[38], &[])
        .expect("record");
    std::fs::write(dir.join("5.json"), "not json").expect("write");
    let fake = FakeSink::new();
    let mut sink = fake.clone();

    HeldJournal::at(dir, ":7", 222)
        .release_stale_where(&mut sink, &dead)
        .expect_err("one record cannot be read");

    assert!(fake.recorded().contains(&Recorded::Key(38, false)));
}

#[test]
fn a_record_of_a_live_process_is_never_called_unreadable() {
    let dir = scratch("livejunk");
    std::fs::create_dir_all(&dir).expect("dir");
    std::fs::write(dir.join("5.json"), "not json").expect("write");
    let fake = FakeSink::new();
    let mut sink = fake.clone();

    HeldJournal::at(dir, ":7", 222)
        .release_stale_where(&mut sink, &alive)
        .expect("a live writer may be mid-write");
}

#[test]
fn a_failed_rewrite_keeps_the_previous_record_and_reports_the_error() {
    let dir = scratch("atomic");
    let journal = HeldJournal::at(dir.clone(), ":7", 111);
    journal.record(&[37], &[]).expect("first record");
    std::fs::create_dir(dir.join("111.json.tmp")).expect("block the temporary file");

    journal
        .record(&[37, 38], &[])
        .expect_err("the rewrite cannot complete");

    let kept = HeldJournal::at(dir, ":7", 222).stale_entries(&dead);
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].keys, [37]);
}

#[test]
fn a_record_that_cannot_be_written_is_an_error() {
    let dir = scratch("blocked");
    std::fs::write(&dir, "a file where the directory belongs").expect("file");

    HeldJournal::at(dir, ":7", 111)
        .record(&[38], &[])
        .expect_err("no directory");
}

#[test]
fn the_journal_file_is_private_to_its_owner() {
    use std::os::unix::fs::PermissionsExt;
    let dir = scratch("mode");
    HeldJournal::at(dir.clone(), ":7", 111)
        .record(&[38], &[])
        .expect("record");

    let mode = std::fs::metadata(dir.join("111.json"))
        .expect("file")
        .permissions()
        .mode();

    assert_eq!(mode & 0o077, 0);
}
