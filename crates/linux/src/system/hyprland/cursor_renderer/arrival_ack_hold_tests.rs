use std::io::{BufRead, BufReader};
use std::os::unix::fs::symlink;
use std::os::unix::net::UnixStream;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use agent_desktop_core::{
    AdapterError, CURSOR_ARRIVAL_TIMEOUT_MS, CursorOverlayConfig, CursorOverlayControl,
    CursorOverlayInstruction, CursorOverlayStyle, ErrorCode, Point,
};
use serde_json::Value;

use super::{
    ARRIVAL_ACK_FLAG, ARRIVAL_ACK_TRACE, ArrivalAckHold, acknowledge_arrival, arrival_ack_hold,
    parse_hold,
};
use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;
use crate::system::hyprland::cursor_renderer::fake_layer_compositor::FakeLayerCompositor;
use crate::system::hyprland::cursor_renderer::registry_fixture::RegistryFixture;
use crate::system::hyprland::cursor_renderer::renderer_process::RendererProcess;
use crate::system::hyprland::cursor_renderer::renderer_request::RendererRequest;
use crate::system::hyprland::cursor_renderer::renderer_target::RendererTarget;
use crate::system::hyprland::cursor_renderer::renderer_wire::exchange;

const SESSION: &str = "fixture-session";

fn flag(fixture: &RegistryFixture, text: &str) {
    std::fs::write(fixture.paths.dir.join(ARRIVAL_ACK_FLAG), text).expect("flag");
}

fn trace(fixture: &RegistryFixture) -> Vec<Value> {
    std::fs::read_to_string(fixture.paths.dir.join(ARRIVAL_ACK_TRACE))
        .unwrap_or_default()
        .lines()
        .map(|line| serde_json::from_str(line).expect("trace line is JSON"))
        .collect()
}

fn reply_within(stream: &UnixStream, wait: Duration) -> Option<Value> {
    stream.set_read_timeout(Some(wait)).expect("timeout");
    let mut line = String::new();
    match BufReader::new(stream).read_line(&mut line) {
        Ok(read) if read > 0 => Some(serde_json::from_str(&line).expect("json reply")),
        _ => None,
    }
}

fn serving(
    fixture: &RegistryFixture,
    fake: &FakeLayerCompositor,
) -> JoinHandle<Result<(), AdapterError>> {
    std::fs::create_dir_all(fixture.root().join("hypr").join("fixture-signature"))
        .expect("compositor dir");
    let target = RendererTarget {
        binding: HyprlandBinding {
            signature: "fixture-signature".into(),
            ipc_socket: fixture.root().join(".socket.sock"),
            wayland_socket: fake.socket.clone(),
            pid: 1,
            start_time: 1,
        },
        runtime_dir: fixture.root().to_path_buf(),
    };
    let enable = CursorOverlayControl::enable(SESSION.into(), CursorOverlayStyle::default());
    let process =
        RendererProcess::start_over(fake.connect(), fixture.paths.clone(), &target, &enable)
            .expect("renderer starts")
            .with_wanted(|_| true)
            .with_multi_agent(|_| false);
    std::thread::spawn(move || process.serve())
}

fn travel(x: f64, y: f64) -> RendererRequest {
    let config = CursorOverlayConfig::enabled(None, 6).expect("config");
    let instruction = CursorOverlayInstruction::new(Point { x, y }, &config, true).expect("instr");
    RendererRequest::control(CursorOverlayControl::present(SESSION.into(), instruction))
}

fn timed_travel(fixture: &RegistryFixture) -> (Result<Option<Value>, AdapterError>, Duration) {
    let started = Instant::now();
    let deadline = started + Duration::from_millis(CURSOR_ARRIVAL_TIMEOUT_MS);
    let reply = exchange(&fixture.paths.socket, &travel(40.0, 40.0), deadline);
    (reply, started.elapsed())
}

fn disable(fixture: &RegistryFixture, renderer: JoinHandle<Result<(), AdapterError>>) {
    let request = RendererRequest::control(CursorOverlayControl::disable(SESSION.into()));
    let deadline = Instant::now() + Duration::from_secs(3);
    exchange(&fixture.paths.socket, &request, deadline).expect("disable");
    renderer.join().expect("thread").expect("clean exit");
}

#[test]
fn the_flag_names_a_delay_in_milliseconds_or_withhold() {
    assert_eq!(
        parse_hold("600\n"),
        Some(ArrivalAckHold::Delay(Duration::from_millis(600)))
    );
    assert_eq!(parse_hold("withhold\n"), Some(ArrivalAckHold::Withhold));
    assert_eq!(parse_hold(""), None);
    assert_eq!(parse_hold("soon"), None);
    assert_eq!(parse_hold("-5"), None);
    assert_eq!(parse_hold("10001"), None);
}

#[test]
fn only_an_owned_regular_flag_file_holds_the_acknowledgement() {
    let fixture = RegistryFixture::new();
    assert_eq!(arrival_ack_hold(&fixture.paths.dir), None);
    flag(&fixture, "withhold");
    assert_eq!(
        arrival_ack_hold(&fixture.paths.dir),
        Some(ArrivalAckHold::Withhold)
    );
    let linked = RegistryFixture::new();
    let target = linked.paths.dir.join("elsewhere");
    std::fs::write(&target, b"withhold").expect("target");
    symlink(&target, linked.paths.dir.join(ARRIVAL_ACK_FLAG)).expect("symlink");
    assert_eq!(arrival_ack_hold(&linked.paths.dir), None);
}

#[test]
fn without_a_flag_the_acknowledgement_goes_at_once_and_nothing_is_traced() {
    let fixture = RegistryFixture::new();
    let (reply, kept) = UnixStream::pair().expect("pair");
    acknowledge_arrival(&fixture.paths.dir, Instant::now(), reply);
    let answer = reply_within(&kept, Duration::from_millis(200)).expect("acknowledged");
    assert_eq!(answer["ok"], true);
    assert!(trace(&fixture).is_empty());
}

#[test]
fn a_delayed_acknowledgement_waits_for_the_delay_after_the_travel_was_received() {
    let fixture = RegistryFixture::new();
    flag(&fixture, "300");
    let (reply, kept) = UnixStream::pair().expect("pair");
    let received = Instant::now();
    acknowledge_arrival(&fixture.paths.dir, received, reply);
    assert!(reply_within(&kept, Duration::from_millis(150)).is_none());
    let answer = reply_within(&kept, Duration::from_secs(1)).expect("acknowledged late");
    assert_eq!(answer["ok"], true);
    assert!(received.elapsed() >= Duration::from_millis(300));
    let lines = trace(&fixture);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert_eq!(lines[0]["hold"], "delay");
    assert_eq!(lines[0]["delay_ms"], 300);
    let received_ns = lines[0]["received_ns"].as_u64().expect("received_ns");
    let acked_ns = lines[0]["acked_ns"].as_u64().expect("acked_ns");
    assert!(acked_ns >= received_ns + 300_000_000, "{lines:?}");
}

#[test]
fn a_withheld_acknowledgement_is_never_sent_and_is_traced() {
    let fixture = RegistryFixture::new();
    flag(&fixture, "withhold");
    let (reply, kept) = UnixStream::pair().expect("pair");
    acknowledge_arrival(&fixture.paths.dir, Instant::now(), reply);
    let wait = Duration::from_millis(CURSOR_ARRIVAL_TIMEOUT_MS + 100);
    assert!(reply_within(&kept, wait).is_none());
    let lines = trace(&fixture);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert_eq!(lines[0]["hold"], "withhold");
    assert!(lines[0]["received_ns"].as_u64().is_some());
    assert!(lines[0]["acked_ns"].is_null());
}

#[test]
fn a_renderer_travel_answers_after_a_delay_inside_the_arrival_timeout() {
    let fixture = RegistryFixture::new();
    let fake = FakeLayerCompositor::start(true);
    let renderer = serving(&fixture, &fake);
    flag(&fixture, "400");
    let (reply, elapsed) = timed_travel(&fixture);
    reply.expect("the delayed travel is acknowledged");
    assert!(elapsed >= Duration::from_millis(400), "{elapsed:?}");
    assert_eq!(trace(&fixture).len(), 1);
    disable(&fixture, renderer);
}

#[test]
fn a_renderer_travel_under_a_withheld_acknowledgement_times_out_at_the_arrival_timeout() {
    let fixture = RegistryFixture::new();
    let fake = FakeLayerCompositor::start(true);
    let renderer = serving(&fixture, &fake);
    flag(&fixture, "withhold");
    let (reply, elapsed) = timed_travel(&fixture);
    let error = reply.expect_err("a withheld travel is never acknowledged");
    assert_eq!(error.code, ErrorCode::Timeout);
    assert!(
        elapsed >= Duration::from_millis(CURSOR_ARRIVAL_TIMEOUT_MS - 50),
        "{elapsed:?}"
    );
    assert_eq!(trace(&fixture)[0]["hold"], "withhold");
    disable(&fixture, renderer);
}
