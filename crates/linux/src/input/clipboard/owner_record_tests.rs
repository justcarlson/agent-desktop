use super::*;
use crate::input::clipboard::test_nonce::test_nonce;

fn record(nonce: &str) -> OwnerRecord {
    OwnerRecord {
        pid: 4242,
        start_ticks: 9_000_123,
        display: ":7".into(),
        window: 0x40_0001,
        nonce: nonce.into(),
    }
}

fn scratch(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("owner-record-{label}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn a_record_whose_start_time_matches_is_live() {
    assert!(is_live_with(&record(&test_nonce()), |pid| {
        (pid == 4242).then_some(9_000_123)
    }));
}

#[test]
fn a_recycled_pid_with_another_start_time_is_stale() {
    assert!(!is_live_with(&record(&test_nonce()), |_| Some(9_000_124)));
}

#[test]
fn a_pid_that_is_gone_is_stale() {
    assert!(!is_live_with(&record(&test_nonce()), |_| None));
}

#[test]
fn pid_zero_is_never_live() {
    let mut zero = record(&test_nonce());
    zero.pid = 0;
    assert!(!is_live_with(&zero, |_| Some(zero.start_ticks)));
}

#[test]
fn start_ticks_of_the_running_process_are_readable_and_stable() {
    let pid = std::process::id();
    let first = start_ticks_of(pid).expect("our own stat reads");
    assert_eq!(start_ticks_of(pid), Some(first));
}

#[test]
fn two_spellings_of_one_display_share_a_name() {
    assert_eq!(normalise_display(":0"), normalise_display(":0.0"));
    assert_ne!(normalise_display(":0"), normalise_display(":0.1"));
    assert_ne!(normalise_display(":0"), normalise_display(":1"));
}

#[test]
fn a_display_name_cannot_escape_the_directory() {
    let name = file_name("../../x:0", &test_nonce()).expect("valid nonce");
    assert!(!name.contains('/'), "{name}");
    assert!(!name.contains(".."), "{name}");
}

#[test]
fn a_nonce_that_is_short_or_has_separators_has_no_file_name() {
    let good = test_nonce();
    let bad = [
        String::new(),
        good[..7].to_owned(),
        format!("{}/", &good[..31]),
        format!("../{}", &good[..28]),
        format!("{good}\n"),
    ];
    for bad in bad {
        assert!(file_name(":0", &bad).is_none(), "{bad:?}");
    }
}

#[test]
fn a_record_survives_its_bytes() {
    let mut original = record(&test_nonce());
    original.display = normalise_display(":7");
    assert_eq!(
        OwnerRecord::from_bytes(&original.to_bytes()),
        Some(original)
    );
}

#[test]
fn a_failure_outcome_carries_its_message_and_is_not_a_record() {
    let dir = scratch("failure");
    let nonce = test_nonce();
    publish_failure(&dir, ":7", &nonce, "payload missing").expect("publish");
    assert_eq!(
        read_outcome(&dir, ":7", &nonce),
        Some(Outcome::Failed("payload missing".into()))
    );
    assert!(records_in(&dir, ":7").is_empty());
    std::fs::remove_dir_all(&dir).expect("cleanup");
}

#[test]
fn records_are_listed_per_display_only() {
    let dir = scratch("listing");
    let seven_nonce = test_nonce();
    let mut seven = record(&seven_nonce);
    seven.display = ":7".into();
    let mut eight = record(&test_nonce());
    eight.display = ":8".into();
    publish(&dir, &seven).expect("seven");
    publish(&dir, &eight).expect("eight");
    let found = records_in(&dir, ":7.0");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].nonce, seven_nonce);
    std::fs::remove_dir_all(&dir).expect("cleanup");
}

#[test]
fn an_exiting_owner_removes_its_own_record() {
    let dir = scratch("own");
    let own = record(&test_nonce());
    publish(&dir, &own).expect("publish");
    remove_own(&dir, &own);
    assert!(records_in(&dir, ":7").is_empty());
    std::fs::remove_dir_all(&dir).expect("cleanup");
}

#[test]
fn a_file_under_our_name_with_another_nonce_is_not_removed() {
    let dir = scratch("foreign");
    let own = record(&test_nonce());
    let name = file_name(&own.display, &own.nonce).expect("name");
    owner_files::ensure_private_dir(&dir).expect("dir");
    let mut stranger = record(&test_nonce());
    stranger.display = normalise_display(":7");
    owner_files::write_new(&dir, &name, &stranger.to_bytes()).expect("plant");
    remove_own(&dir, &own);
    assert!(dir.join(&name).exists());
    std::fs::remove_dir_all(&dir).expect("cleanup");
}
