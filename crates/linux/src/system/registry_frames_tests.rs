use std::time::{Duration, Instant};

use async_io::Timer;

use super::collect_frames;

fn children(count: usize) -> Vec<(String, String)> {
    (0..count)
        .map(|index| (format!(":1.{index}"), format!("/frame/{index}")))
        .collect()
}

#[test]
fn one_stalled_frame_costs_one_slice_and_keeps_its_peers() {
    let started = Instant::now();
    let frames = zbus::block_on(collect_frames(
        children(4),
        Duration::from_millis(100),
        |(bus, _)| async move {
            if bus == ":1.1" {
                Timer::after(Duration::from_secs(30)).await;
            }
            Some(bus)
        },
    ));
    assert_eq!(frames, [":1.0", ":1.2", ":1.3"]);
    assert!(started.elapsed() < Duration::from_secs(5));
}

#[test]
fn a_frame_that_fails_to_read_is_left_out() {
    let frames = zbus::block_on(collect_frames(
        children(3),
        Duration::from_millis(100),
        |(bus, _)| async move { (bus != ":1.0").then_some(bus) },
    ));
    assert_eq!(frames, [":1.1", ":1.2"]);
}
