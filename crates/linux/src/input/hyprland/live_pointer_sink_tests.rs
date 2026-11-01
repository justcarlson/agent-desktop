use std::collections::VecDeque;
use std::sync::atomic::Ordering;
use std::time::Instant;

use agent_desktop_core::DeliverySemantics;

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

fn reads(points: &[(f64, f64)]) -> CursorRead {
    let mut replies: VecDeque<Point> = points.iter().map(|&(x, y)| Point { x, y }).collect();
    Box::new(move || {
        replies
            .pop_front()
            .ok_or_else(|| AdapterError::internal("The Hyprland cursor position could not be read"))
    })
}

fn opened_sink(compositor: &FakeCompositor, cursor: CursorRead) -> LivePointerSink {
    let stream = UnixStream::connect(&compositor.socket).expect("connects");
    let deadline = Deadline::after(2_000).expect("deadline");
    LivePointerSink::over(stream, compositor.socket.clone(), deadline, cursor).expect("opens")
}

fn refusal_cause(error: &AdapterError) -> Option<&str> {
    error.details.as_ref()?.get("cause")?.as_str()
}

fn point_at(x: u32, y: u32) -> OutputPoint {
    OutputPoint {
        output: "DP-1".to_owned(),
        x,
        y,
        x_extent: 1920 * 120,
        y_extent: 1080 * 120,
        global: ((x / 120) as i32, (y / 120) as i32),
    }
}

fn pressed_sink(compositor: &FakeCompositor) -> LivePointerSink {
    let mut sink = opened_sink(compositor, reads(&[(0.0, 0.0), (1.0, 1.0)]));
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
    const ONE_MOTION: usize = 2;
    let compositor = compositor();
    let mut sink = opened_sink(
        &compositor,
        reads(&[(953.0, 81.0), (953.0, 81.0), (953.0, 81.0), (960.0, 90.0)]),
    );
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
fn the_first_motion_passes_beside_the_aim_inside_the_output() {
    assert_eq!(beside_dx(953 * 120), -1);
    assert_eq!(beside_dx(120), -1);
    assert_eq!(beside_dx(60), 1);
    assert_eq!(beside_dx(0), 1);
}

#[test]
fn a_motion_that_misses_the_aim_refuses_before_any_button_or_wheel() {
    let compositor = compositor();
    let mut sink = opened_sink(&compositor, reads(&[(0.0, 0.0), (6.0, 1.0)]));
    let error = sink
        .motion(&point_at(120, 120))
        .expect_err("a cursor Hyprland moved elsewhere than the aim must refuse");
    assert_eq!(refusal_cause(&error), Some("cursor_missed_aim"));
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
}

#[test]
fn a_motion_with_no_cursor_evidence_refuses() {
    for replies in [&[][..], &[(0.0, 0.0)][..]] {
        let compositor = compositor();
        let mut sink = opened_sink(&compositor, reads(replies));
        let error = sink
            .motion(&point_at(120, 120))
            .expect_err("a motion whose start or landing is unread must refuse");
        assert_eq!(refusal_cause(&error), Some("cursor_unread"));
        assert_eq!(error.disposition, DeliverySemantics::not_delivered());
    }
}
