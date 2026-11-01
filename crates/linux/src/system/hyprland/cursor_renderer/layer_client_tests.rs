use agent_desktop_core::{CursorOverlayStyle, Deadline};

use super::LayerClient;
use crate::system::hyprland::cursor_renderer::arrow_bitmap::ArrowBitmap;
use crate::system::hyprland::cursor_renderer::fake_layer_compositor::FakeLayerCompositor;

fn deadline() -> Deadline {
    Deadline::detached_after(2_000).expect("deadline")
}

fn drawn(fake: &FakeLayerCompositor) -> LayerClient {
    let mut client = LayerClient::over(fake.connect(), &fake.socket, deadline()).expect("client");
    let bitmap = ArrowBitmap::draw(&CursorOverlayStyle::default());
    client.draw(&bitmap, (100, 200), deadline()).expect("draw");
    client
}

#[test]
fn the_cursor_is_an_overlay_layer_that_takes_no_input_and_moves_nothing() {
    let fake = FakeLayerCompositor::start(true);
    let client = drawn(&fake);
    let log = fake.logged();
    for expected in [
        "layer 3 namespace agent-desktop-cursor output 0",
        "input region empty",
        "keyboard 0",
        "exclusive zone -1",
        "anchor 5",
        "ack configure",
        "margin 200 0 0 100",
    ] {
        assert!(
            log.iter().any(|line| line == expected),
            "{expected} missing from {log:?}"
        );
    }
    assert_eq!(client.output_count(), 1);
}

#[test]
fn the_renderer_binds_no_seat_and_no_activation_global() {
    let fake = FakeLayerCompositor::start(true);
    let _client = drawn(&fake);
    let binds: Vec<String> = fake
        .logged()
        .into_iter()
        .filter(|line| line.starts_with("bind "))
        .collect();
    assert_eq!(
        binds,
        [
            "bind wl_compositor",
            "bind wl_shm",
            "bind zwlr_layer_shell_v1"
        ]
    );
}

#[test]
fn a_compositor_that_shows_nothing_still_lets_a_draw_finish() {
    let fake = FakeLayerCompositor::start(false);
    let mut client = LayerClient::over(fake.connect(), &fake.socket, deadline()).expect("client");
    let bitmap = ArrowBitmap::draw(&CursorOverlayStyle::default());
    let quick = Deadline::detached_after(100).expect("deadline");
    client
        .draw(&bitmap, (5, 5), quick)
        .expect("a dark output is not an error");
    client
        .clear(Deadline::detached_after(100).expect("deadline"))
        .expect("clear");
}

#[test]
fn release_destroys_the_layer_surface() {
    let fake = FakeLayerCompositor::start(true);
    let mut client = drawn(&fake);
    client.release();
    client.roundtrip(deadline()).expect("roundtrip");
    assert!(fake.logged().iter().any(|line| line == "destroy layer"));
}
