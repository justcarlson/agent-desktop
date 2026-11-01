use agent_desktop_core::{CursorOverlayStyle, Deadline, Point};

use super::LayerClient;
use crate::system::hyprland::cursor_renderer::agent_cursor::AgentCursor;
use crate::system::hyprland::cursor_renderer::arrow_bitmap::ArrowBitmap;
use crate::system::hyprland::cursor_renderer::fake_layer_compositor::FakeLayerCompositor;
use crate::system::hyprland::cursor_renderer::surface_key::SurfaceKey;

fn key(cursor: &str) -> SurfaceKey {
    SurfaceKey {
        cursor: cursor.into(),
        output: None,
    }
}

fn deadline() -> Deadline {
    Deadline::detached_after(2_000).expect("deadline")
}

fn drawn(fake: &FakeLayerCompositor) -> LayerClient {
    let mut client = LayerClient::over(fake.connect(), &fake.socket, deadline()).expect("client");
    let bitmap = ArrowBitmap::draw(&CursorOverlayStyle::default());
    client
        .draw(&key(""), &bitmap, (100, 200), deadline())
        .expect("draw");
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
        .draw(&key(""), &bitmap, (5, 5), quick)
        .expect("a dark output is not an error");
    client
        .clear("", Deadline::detached_after(100).expect("deadline"))
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

#[test]
fn frame_marks_keep_rising_when_the_surface_is_rebuilt() {
    let fake = FakeLayerCompositor::start(true);
    let mut client = drawn(&fake);
    let bitmap = ArrowBitmap::draw(&CursorOverlayStyle::default());
    client
        .draw(&key(""), &bitmap, (110, 210), deadline())
        .expect("second frame");
    client.release();
    client
        .draw(&key(""), &bitmap, (120, 220), deadline())
        .expect("rebuilt frame");
    client.roundtrip(deadline()).expect("roundtrip");
    assert_eq!(
        client.state.frames_done, 3,
        "the rebuilt surface's first frame reused an old frame mark"
    );
}

#[test]
fn each_cursor_key_gets_its_own_surface_and_forgetting_one_keeps_the_other() {
    let fake = FakeLayerCompositor::start(true);
    let mut client = LayerClient::over(fake.connect(), &fake.socket, deadline()).expect("client");
    let bitmap = ArrowBitmap::draw(&CursorOverlayStyle::default());
    client
        .draw(&key("a"), &bitmap, (10, 20), deadline())
        .expect("a");
    client
        .draw(&key("b"), &bitmap, (30, 40), deadline())
        .expect("b");
    assert_eq!(client.surface_count(), 2);
    client.forget("a");
    client.roundtrip(deadline()).expect("roundtrip");
    assert_eq!(client.surface_count(), 1);
    let destroyed = fake
        .logged()
        .iter()
        .filter(|line| *line == "destroy layer")
        .count();
    assert_eq!(destroyed, 1);
}

const LEFT: [i32; 6] = [-1024, 0, 1024, 768, 1280, 960];
const RIGHT: [i32; 6] = [0, 0, 1280, 800, 1280, 800];

fn cursor() -> AgentCursor {
    let style = CursorOverlayStyle::default();
    AgentCursor {
        bitmap: ArrowBitmap::draw(&style),
        style,
        motion: Default::default(),
        pose: None,
        moves: 0,
        hidden: false,
        travel: None,
    }
}

fn laid_out() -> (FakeLayerCompositor, LayerClient) {
    let fake = FakeLayerCompositor::start_with_outputs(true, vec![LEFT, RIGHT]);
    let client = LayerClient::over(fake.connect(), &fake.socket, deadline()).expect("client");
    (fake, client)
}

fn staged(client: &mut LayerClient, x: f64, y: f64) {
    let mark = client
        .stage_point("a", &cursor(), &Point { x, y }, deadline())
        .expect("stage");
    client.settle(mark, deadline()).expect("settle");
}

fn output_id(log: &[String], index: usize) -> String {
    let prefix = format!("output {index} is ");
    log.iter()
        .find_map(|line| line.strip_prefix(&prefix).map(str::to_owned))
        .unwrap_or_else(|| panic!("output {index} never bound: {log:?}"))
}

#[test]
fn a_cursor_across_an_output_edge_gets_a_surface_pinned_to_each_output_at_its_scale() {
    let (fake, mut client) = laid_out();
    staged(&mut client, -5.0, 100.0);
    let log = fake.logged();
    let (left, right) = (output_id(&log, 0), output_id(&log, 1));
    for expected in [
        format!("layer 3 namespace agent-desktop-cursor output {left}"),
        format!("layer 3 namespace agent-desktop-cursor output {right}"),
        "size 28 36".to_owned(),
        "viewport 28 36".to_owned(),
        "margin 99 0 0 1018".to_owned(),
        "size 25 33".to_owned(),
        "viewport 25 33".to_owned(),
        "margin 99 0 0 -7".to_owned(),
    ] {
        assert!(log.contains(&expected), "{expected} missing from {log:?}");
    }
    assert_eq!(client.surface_count(), 2);
}

#[test]
fn a_cursor_inside_one_output_draws_only_there_and_leaves_the_other_clear() {
    let (fake, mut client) = laid_out();
    staged(&mut client, -5.0, 100.0);
    staged(&mut client, 500.0, 500.0);
    let log = fake.logged();
    assert!(log.contains(&"margin 499 0 0 499".to_owned()), "{log:?}");
    assert_eq!(client.surface_count(), 2, "the left surface stays, cleared");
}

#[test]
fn a_point_on_no_output_is_off_the_layout_and_builds_no_surface() {
    let (fake, mut client) = laid_out();
    assert!(client.on_output(&Point { x: -1.0, y: 700.0 }));
    assert!(!client.on_output(&Point {
        x: 5000.0,
        y: 5000.0
    }));
    assert!(!client.on_output(&Point {
        x: 10.0,
        y: 790.0 + 20.0
    }));
    staged(&mut client, 5000.0, 5000.0);
    assert_eq!(client.surface_count(), 0);
    assert!(
        !fake.logged().iter().any(|line| line.starts_with("layer ")),
        "{:?}",
        fake.logged()
    );
}

#[test]
fn the_layout_globals_add_outputs_and_a_viewporter_but_never_a_seat() {
    let (fake, _client) = laid_out();
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
            "bind zwlr_layer_shell_v1",
            "bind zxdg_output_manager_v1",
            "bind wp_viewporter",
            "bind wl_output",
            "bind wl_output"
        ]
    );
}

#[test]
fn an_arrival_settles_only_once_the_output_it_is_drawn_on_shows_it() {
    let fake = FakeLayerCompositor::start_with_slow_output(true, vec![LEFT, RIGHT], Some(1));
    let mut client = LayerClient::over(fake.connect(), &fake.socket, deadline()).expect("client");
    staged(&mut client, -5.0, 100.0);
    let shown_before = shown_on_slow_output(&fake);
    staged(&mut client, 500.0, 500.0);
    assert!(
        shown_on_slow_output(&fake) > shown_before,
        "the draw settled on the left output's clear frame before the right output showed the \
         arrow: {:?}",
        fake.logged()
    );
}

fn shown_on_slow_output(fake: &FakeLayerCompositor) -> usize {
    fake.logged()
        .iter()
        .filter(|line| *line == "slow output shown")
        .count()
}
