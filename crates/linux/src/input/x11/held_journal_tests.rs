use std::path::PathBuf;

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
    HeldJournal::at(dir.clone(), ":7", 111).record(&[38], &[1]);
    let next = HeldJournal::at(dir.clone(), ":7", 222);
    let fake = FakeSink::new();
    let mut sink = fake.clone();

    next.release_stale_where(&mut sink, &dead);

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
    HeldJournal::at(dir.clone(), ":7", 111).record(&[38], &[]);
    let fake = FakeSink::new();
    let mut sink = fake.clone();

    HeldJournal::at(dir, ":7", 222).release_stale_where(&mut sink, &alive);

    assert!(fake.recorded().is_empty());
}

#[test]
fn input_held_on_another_display_is_left_alone() {
    let dir = scratch("display");
    HeldJournal::at(dir.clone(), ":9", 111).record(&[38], &[]);
    let fake = FakeSink::new();
    let mut sink = fake.clone();

    HeldJournal::at(dir, ":7", 222).release_stale_where(&mut sink, &dead);

    assert!(fake.recorded().is_empty());
}

#[test]
fn a_call_never_releases_its_own_entry_as_stale() {
    let dir = scratch("own");
    let journal = HeldJournal::at(dir, ":7", 111);
    journal.record(&[38], &[]);

    assert!(journal.stale_entries(&dead).is_empty());
}

#[test]
fn recording_nothing_held_removes_the_entry() {
    let dir = scratch("clear");
    let journal = HeldJournal::at(dir.clone(), ":7", 111);
    journal.record(&[38], &[]);
    journal.record(&[], &[]);

    assert!(
        HeldJournal::at(dir, ":7", 222)
            .stale_entries(&dead)
            .is_empty()
    );
}

#[test]
fn an_unreadable_entry_is_ignored() {
    let dir = scratch("junk");
    std::fs::create_dir_all(&dir).expect("dir");
    std::fs::write(dir.join("5.json"), "not json").expect("write");

    assert!(
        HeldJournal::at(dir, ":7", 222)
            .stale_entries(&dead)
            .is_empty()
    );
}

#[test]
fn the_journal_file_is_private_to_its_owner() {
    use std::os::unix::fs::PermissionsExt;
    let dir = scratch("mode");
    HeldJournal::at(dir.clone(), ":7", 111).record(&[38], &[]);

    let mode = std::fs::metadata(dir.join("111.json"))
        .expect("file")
        .permissions()
        .mode();

    assert_eq!(mode & 0o077, 0);
}
