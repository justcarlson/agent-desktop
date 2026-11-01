use std::sync::atomic::Ordering;
use std::time::Instant;

use super::*;
use crate::system::wayland::fake_compositor::{FakeCompositor, FakeOutput, Script};

const BTN_LEFT: u32 = 0x110;

fn compositor() -> FakeCompositor {
    FakeCompositor::start(Script::Answer {
        globals: vec![
            (MANAGER, MANAGER_VERSION),
            ("wl_seat", 1),
            ("wl_output", OUTPUT_VERSION),
        ],
        outputs: vec![FakeOutput {
            name: "DP-1",
            position: (0, 0),
            logical: (1920, 1080),
            mode: (1920, 1080),
            transform: 0,
            scale: 1,
        }],
    })
}

fn opened_sink(compositor: &FakeCompositor) -> LivePointerSink {
    let stream = UnixStream::connect(&compositor.socket).expect("connects");
    let deadline = Deadline::after(2_000).expect("deadline");
    LivePointerSink::over(stream, compositor.socket.clone(), deadline).expect("opens")
}

fn point_at(x: u32, y: u32) -> OutputPoint {
    OutputPoint {
        output: "DP-1".to_owned(),
        x,
        y,
        x_extent: 1920 * 120,
        y_extent: 1080 * 120,
    }
}

fn pressed_sink(compositor: &FakeCompositor) -> LivePointerSink {
    let mut sink = opened_sink(compositor);
    sink.motion(&point_at(120, 120)).expect("moves");
    sink.button(BTN_LEFT, true).expect("presses");
    sink
}

fn requests_for(
    compositor: &FakeCompositor,
    sink: &mut LivePointerSink,
    point: &OutputPoint,
) -> usize {
    sink.finish().expect("settles");
    let before = compositor.requests.load(Ordering::SeqCst);
    sink.motion(point).expect("moves");
    sink.finish().expect("settles");
    compositor.requests.load(Ordering::SeqCst) - before
}

#[test]
fn the_first_motion_moves_the_cursor_even_when_it_already_rests_on_the_aim() {
    const POINTER_CREATION: usize = 2;
    const ONE_MOTION: usize = 3;
    let compositor = compositor();
    let mut sink = opened_sink(&compositor);
    let first = requests_for(&compositor, &mut sink, &point_at(953 * 120, 81 * 120));
    let later = requests_for(&compositor, &mut sink, &point_at(960 * 120, 90 * 120));
    assert_eq!(
        first,
        later + POINTER_CREATION + ONE_MOTION,
        "Hyprland drops a motion to the pixel the cursor is on, so a single first motion leaves pointer focus on the window that was there before"
    );
}

#[test]
fn a_recovered_release_is_read_by_the_compositor_before_recover_returns() {
    let compositor = compositor();
    let mut sink = pressed_sink(&compositor);
    let before = compositor.requests.load(Ordering::SeqCst);
    assert!(sink.recover(&[BTN_LEFT]));
    assert!(
        compositor.requests.load(Ordering::SeqCst) >= before + 2,
        "the release and its frame were not read before recover returned"
    );
}

#[test]
fn a_release_the_compositor_never_confirms_is_not_reported_released() {
    let compositor = compositor();
    let mut sink = pressed_sink(&compositor);
    compositor.stalled.store(true, Ordering::SeqCst);
    let started = Instant::now();
    assert!(
        !sink.recover(&[BTN_LEFT]),
        "a release only written to the socket was reported released; a client that exits next loses it"
    );
    assert!(
        started.elapsed() < Duration::from_millis(1_900),
        "{:?}",
        started.elapsed()
    );
}

#[test]
fn the_first_motion_starts_inside_the_output_at_its_left_edge() {
    assert_eq!(one_pixel_beside(953 * 120), 952 * 120);
    assert_eq!(one_pixel_beside(120), 0);
    assert_eq!(one_pixel_beside(60), 180);
    assert_eq!(one_pixel_beside(0), 120);
}

#[test]
fn every_absolute_motion_shares_its_frame_with_a_still_relative_motion() {
    use MotionRequest::{Absolute, Frame, StillRelative};
    assert_eq!(
        motion_requests(953 * 120, true),
        vec![
            StillRelative,
            Absolute { x: 952 * 120 },
            Frame,
            StillRelative,
            Absolute { x: 953 * 120 },
            Frame
        ],
        "XWayland sends an absolute-only motion through its absolute pointer device and the wheel through its relative one, so the switch resets GDK3's scroll baseline and the next notch scrolls nothing"
    );
    assert_eq!(
        motion_requests(960 * 120, false),
        vec![StillRelative, Absolute { x: 960 * 120 }, Frame]
    );
}
