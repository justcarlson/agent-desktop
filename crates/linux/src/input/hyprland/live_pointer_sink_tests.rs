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

fn pressed_sink(compositor: &FakeCompositor) -> LivePointerSink {
    let stream = UnixStream::connect(&compositor.socket).expect("connects");
    let deadline = Deadline::after(2_000).expect("deadline");
    let mut sink =
        LivePointerSink::over(stream, compositor.socket.clone(), deadline).expect("opens");
    let point = OutputPoint {
        output: "DP-1".to_owned(),
        x: 120,
        y: 120,
        x_extent: 1920 * 120,
        y_extent: 1080 * 120,
    };
    sink.motion(&point).expect("moves");
    sink.button(BTN_LEFT, true).expect("presses");
    sink
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
