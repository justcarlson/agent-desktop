use std::time::{Duration, Instant};

use agent_desktop_core::{AdapterError, ErrorCode};
use async_io::Timer;

use super::{RootRead, collect_frames};

fn children(count: usize) -> Vec<(String, String)> {
    (0..count)
        .map(|index| (format!(":1.{index}"), format!("/frame/{index}")))
        .collect()
}

fn gone() -> AdapterError {
    AdapterError::new(ErrorCode::ElementNotFound, "gone")
}

#[test]
fn one_stalled_frame_costs_one_slice_keeps_its_peers_and_reads_timed_out() {
    let started = Instant::now();
    let (frames, read) = zbus::block_on(collect_frames(
        children(4),
        Duration::from_millis(100),
        || false,
        |_| false,
        |(bus, _)| async move {
            if bus == ":1.1" {
                Timer::after(Duration::from_secs(30)).await;
            }
            Ok(bus)
        },
    ));
    assert_eq!(frames, [":1.0", ":1.2", ":1.3"]);
    assert_eq!(read, RootRead::TimedOut);
    assert!(started.elapsed() < Duration::from_secs(5));
}

#[test]
fn a_frame_that_fails_for_another_reason_reads_timed_out() {
    let (frames, read) = zbus::block_on(collect_frames(
        children(3),
        Duration::from_millis(100),
        || false,
        |_| false,
        |(bus, _)| async move {
            if bus == ":1.0" {
                Err(AdapterError::internal("failed field"))
            } else {
                Ok(bus)
            }
        },
    ));
    assert_eq!(frames, [":1.1", ":1.2"]);
    assert_eq!(read, RootRead::TimedOut);
}

#[test]
fn a_gone_frame_is_left_out_and_the_read_is_complete() {
    let (frames, read) = zbus::block_on(collect_frames(
        children(3),
        Duration::from_millis(100),
        || false,
        |_| false,
        |(bus, _)| async move { if bus == ":1.0" { Err(gone()) } else { Ok(bus) } },
    ));
    assert_eq!(frames, [":1.1", ":1.2"]);
    assert_eq!(read, RootRead::Read);
}

#[test]
fn gone_and_timed_out_never_compare_equal() {
    assert_ne!(RootRead::Read, RootRead::TimedOut);
}

#[test]
fn every_gone_code_is_an_absence() {
    for code in [
        ErrorCode::WindowNotFound,
        ErrorCode::AppNotFound,
        ErrorCode::ElementNotFound,
    ] {
        assert!(super::is_gone(&AdapterError::new(code, "x")));
    }
    assert!(!super::is_gone(&AdapterError::internal("x")));
    assert!(!super::is_gone(&AdapterError::new(ErrorCode::Timeout, "x")));
}

#[test]
fn a_child_list_that_does_not_answer_is_slow_and_a_gone_one_is_not() {
    use super::{Outcome, outcome};
    assert!(matches!(outcome::<u8>(None, || false), Outcome::Slow));
    assert!(matches!(
        outcome::<u8>(Some(Err(gone())), || false),
        Outcome::Gone
    ));
    assert!(matches!(
        outcome::<u8>(Some(Err(AdapterError::internal("x"))), || false),
        Outcome::Slow
    ));
    assert!(matches!(outcome(Some(Ok(1u8)), || false), Outcome::Read(1)));
}

#[test]
fn a_failure_while_the_process_is_gone_is_an_absence_and_while_it_lives_is_slow() {
    use super::{Outcome, outcome};
    let failed = || Some(Err::<u8, _>(AdapterError::internal("NoReply")));
    assert!(matches!(outcome(failed(), || true), Outcome::Gone));
    assert!(matches!(outcome(failed(), || false), Outcome::Slow));
    assert!(matches!(outcome::<u8>(None, || true), Outcome::Slow));
}

#[test]
fn a_frame_that_fails_after_its_process_left_reads_complete() {
    let (frames, read) = zbus::block_on(collect_frames(
        children(2),
        Duration::from_millis(100),
        || true,
        |_| false,
        |(bus, _)| async move {
            if bus == ":1.0" {
                Err(AdapterError::internal("NoReply"))
            } else {
                Ok(bus)
            }
        },
    ));
    assert_eq!(frames, [":1.1"]);
    assert_eq!(read, RootRead::Read);
}

#[test]
fn a_frame_whose_classifying_field_failed_is_kept_and_marks_the_read_incomplete() {
    use crate::tree::test_support::read_of;
    use crate::tree::{Field, RawNode};
    let whole = read_of(23, "w", &[]).node;
    let mut interfaces_failed = whole.clone();
    interfaces_failed.traits.interfaces = Field::Failed("GetInterfaces".to_owned());
    let (frames, read) = zbus::block_on(collect_frames(
        children(2),
        Duration::from_millis(100),
        || false,
        super::is_unclassified,
        |(bus, _)| {
            let node: RawNode = if bus == ":1.0" {
                interfaces_failed.clone()
            } else {
                whole.clone()
            };
            async move { Ok(node) }
        },
    ));
    assert_eq!(frames.len(), 2);
    assert_eq!(read, RootRead::TimedOut);
    assert!(!super::is_unclassified(&whole));
}
