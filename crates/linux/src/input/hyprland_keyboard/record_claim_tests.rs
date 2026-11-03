use std::path::PathBuf;

use agent_desktop_core::Deadline;

use super::claim_record;

fn record(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ad-record-claim-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the folder is created");
    let path = dir.join("77.json");
    std::fs::write(&path, r#"{"keys":[30]}"#).expect("the record writes");
    path
}

fn brief() -> Deadline {
    Deadline::after(50).expect("a short deadline builds")
}

#[test]
fn a_missing_record_gives_no_claim() {
    let path = record("missing");
    std::fs::remove_file(&path).expect("the record is removed");
    assert!(claim_record(&path, brief()).expect("no error").is_none());
}

#[test]
fn a_second_claim_waits_and_then_times_out_while_the_first_is_held() {
    let path = record("held");
    let first = claim_record(&path, brief())
        .expect("no error")
        .expect("the first claim holds the record");
    let error = claim_record(&path, brief()).expect_err("the record is claimed");
    assert_eq!(error.kind(), std::io::ErrorKind::TimedOut);
    assert_eq!(first.value().expect("the record reads")["keys"][0], 30);
}

#[test]
fn a_record_the_first_claim_removed_gives_the_second_nothing() {
    let path = record("removed");
    let first = claim_record(&path, brief())
        .expect("no error")
        .expect("the first claim holds the record");
    first.remove().expect("the record is removed");
    assert!(claim_record(&path, brief()).expect("no error").is_none());
}

#[test]
fn a_released_claim_that_kept_the_record_lets_the_next_claim_take_it() {
    let path = record("kept");
    drop(
        claim_record(&path, brief())
            .expect("no error")
            .expect("claimed"),
    );
    assert!(claim_record(&path, brief()).expect("no error").is_some());
}
