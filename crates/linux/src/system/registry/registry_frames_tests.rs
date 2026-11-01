use std::time::{Duration, Instant};

use agent_desktop_core::{AdapterError, ErrorCode};
use async_io::Timer;

use super::{RootRead, collect_frames, read_app};
use crate::system::registry::app_patience::AppPatience;
use crate::system::registry::read_patience::ReadPatience;

const SLICE: Duration = Duration::from_millis(100);
const GRACE: Duration = Duration::from_millis(400);

fn stopped() -> ReadPatience {
    ReadPatience::for_state(SLICE, GRACE, true)
}

fn running() -> ReadPatience {
    ReadPatience::for_state(SLICE, GRACE, false)
}

fn children(count: usize) -> Vec<(String, String)> {
    (0..count)
        .map(|index| (format!(":1.{index}"), format!("/frame/{index}")))
        .collect()
}

#[test]
fn one_stalled_frame_costs_one_slice_keeps_its_peers_and_reads_timed_out() {
    let started = Instant::now();
    let (frames, read) = zbus::block_on(collect_frames(
        children(4),
        stopped(),
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
    for code in [ErrorCode::Internal, ErrorCode::Timeout] {
        let (frames, read) = zbus::block_on(collect_frames(
            children(2),
            stopped(),
            |_| false,
            move |(bus, _)| {
                let code = code.clone();
                async move {
                    if bus == ":1.0" {
                        Err(AdapterError::new(code, "failed field"))
                    } else {
                        Ok(bus)
                    }
                }
            },
        ));
        assert_eq!(frames, [":1.1"]);
        assert_eq!(read, RootRead::TimedOut);
    }
}

#[test]
fn a_gone_frame_is_left_out_and_the_read_is_complete() {
    for code in [
        ErrorCode::WindowNotFound,
        ErrorCode::AppNotFound,
        ErrorCode::ElementNotFound,
    ] {
        let (frames, read) = zbus::block_on(collect_frames(
            children(3),
            stopped(),
            |_| false,
            move |(bus, _)| {
                let code = code.clone();
                async move {
                    if bus == ":1.0" {
                        Err(AdapterError::new(code, "gone"))
                    } else {
                        Ok(bus)
                    }
                }
            },
        ));
        assert_eq!(frames, [":1.1", ":1.2"]);
        assert_eq!(read, RootRead::Read);
    }
}

#[test]
fn a_failed_frame_read_keeps_the_inventory_incomplete() {
    let (frames, read) = zbus::block_on(collect_frames(
        children(2),
        stopped(),
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
    assert_eq!(read, RootRead::TimedOut);
}

#[test]
fn a_frame_whose_classifying_field_failed_is_kept_and_marks_the_read_incomplete() {
    use crate::tree::live::test_support::read_of;
    use crate::tree::{Field, RawNode};
    let whole = read_of(23, "w", &[]).node;
    let mut interfaces_failed = whole.clone();
    interfaces_failed.traits.interfaces = Field::Failed("GetInterfaces".to_owned());
    let (frames, read) = zbus::block_on(collect_frames(
        children(2),
        stopped(),
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

#[test]
fn a_running_app_frame_that_answers_after_its_slice_is_kept_and_reads_complete() {
    let (frames, read) = zbus::block_on(collect_frames(
        children(2),
        running(),
        |_| false,
        |(bus, _)| async move {
            if bus == ":1.0" {
                Timer::after(SLICE * 2).await;
            }
            Ok(bus)
        },
    ));
    assert_eq!(frames, [":1.0", ":1.1"]);
    assert_eq!(read, RootRead::Read);
}

#[test]
fn a_running_app_child_list_that_answers_after_its_slice_is_read() {
    let patience = AppPatience {
        children: running(),
        frames: running(),
    };
    let (frames, read) = zbus::block_on(read_app(
        patience,
        async {
            Timer::after(SLICE * 2).await;
            Ok(children(2))
        },
        |_| false,
        |(bus, _)| async move { Ok(bus) },
    ));
    assert_eq!(frames, [":1.0", ":1.1"]);
    assert_eq!(read, RootRead::Read);
}

#[test]
fn a_stopped_app_child_list_costs_one_slice_and_reads_timed_out() {
    let patience = AppPatience {
        children: stopped(),
        frames: stopped(),
    };
    let started = Instant::now();
    let (frames, read) = zbus::block_on(read_app(
        patience,
        async {
            Timer::after(Duration::from_secs(30)).await;
            Ok(children(2))
        },
        |_| false,
        |(bus, _)| async move { Ok::<_, AdapterError>(bus) },
    ));
    assert!(frames.is_empty());
    assert_eq!(read, RootRead::TimedOut);
    assert!(started.elapsed() < SLICE + GRACE);
}

#[test]
fn a_running_app_that_never_answers_costs_its_slice_and_grace_and_reads_timed_out() {
    let started = Instant::now();
    let (frames, read) = zbus::block_on(collect_frames(
        children(1),
        running(),
        |_| false,
        |(bus, _)| async move {
            Timer::after(Duration::from_secs(30)).await;
            Ok(bus)
        },
    ));
    assert!(frames.is_empty());
    assert_eq!(read, RootRead::TimedOut);
    assert!(started.elapsed() >= SLICE + GRACE);
    assert!(started.elapsed() < Duration::from_secs(5));
}
