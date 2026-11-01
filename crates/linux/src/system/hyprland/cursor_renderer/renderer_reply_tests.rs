use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use agent_desktop_core::{
    AdapterError, CursorOverlayConfig, CursorOverlayControl, CursorOverlayInstruction,
    CursorOverlayStyle, Point,
};
use serde_json::Value;

use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;
use crate::system::hyprland::cursor_renderer::fake_layer_compositor::FakeLayerCompositor;
use crate::system::hyprland::cursor_renderer::registry_fixture::RegistryFixture;
use crate::system::hyprland::cursor_renderer::renderer_process::RendererProcess;
use crate::system::hyprland::cursor_renderer::renderer_request::RendererRequest;
use crate::system::hyprland::cursor_renderer::renderer_target::RendererTarget;
use crate::system::hyprland::cursor_renderer::renderer_wire::exchange;

const SESSION: &str = "fixture-session";

fn serving(
    fixture: &RegistryFixture,
    fake: &FakeLayerCompositor,
    first: Option<&str>,
    multi_agent: fn(&str) -> bool,
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
    let enable = CursorOverlayControl::enable(SESSION.into(), CursorOverlayStyle::default())
        .with_agent_id(first.map(str::to_owned));
    let process =
        RendererProcess::start_over(fake.connect(), fixture.paths.clone(), &target, &enable)
            .expect("renderer starts")
            .with_wanted(|_| true)
            .with_multi_agent(multi_agent);
    std::thread::spawn(move || process.serve())
}

fn travel(agent: Option<&str>, x: f64, y: f64) -> RendererRequest {
    let config = CursorOverlayConfig::enabled(None, 6).expect("config");
    let instruction = CursorOverlayInstruction::new(Point { x, y }, &config, true).expect("instr");
    RendererRequest::control(
        CursorOverlayControl::present(SESSION.into(), instruction)
            .with_agent_id(agent.map(str::to_owned)),
    )
}

fn send(fixture: &RegistryFixture, request: &RendererRequest) -> Result<Value, AdapterError> {
    let deadline = Instant::now() + Duration::from_secs(3);
    exchange(&fixture.paths.socket, request, deadline).map(|reply| reply.expect("listens"))
}

fn sent(fixture: &RegistryFixture, request: &RendererRequest) -> UnixStream {
    let mut stream = UnixStream::connect(&fixture.paths.socket).expect("connect");
    let payload = request.to_json().expect("json").to_string();
    stream.write_all(payload.as_bytes()).expect("write");
    stream
        .shutdown(std::net::Shutdown::Write)
        .expect("shutdown");
    stream
}

fn reply_kind(stream: UnixStream) -> Value {
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .expect("timeout");
    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line).expect("reply");
    let reply: Value = serde_json::from_str(&line).expect("json reply");
    reply["error"]["kind"].clone()
}

fn disable() -> RendererRequest {
    RendererRequest::control(CursorOverlayControl::disable(SESSION.into()))
}

#[test]
fn a_newer_present_for_the_same_agent_supersedes_its_travel_in_flight() {
    let fixture = RegistryFixture::new();
    let fake = FakeLayerCompositor::start(true);
    let renderer = serving(&fixture, &fake, Some("agent-a"), |_| true);
    let first = sent(&fixture, &travel(Some("agent-a"), 1500.0, 900.0));
    send(&fixture, &travel(Some("agent-a"), 40.0, 40.0)).expect("the newer travel arrives");
    assert_eq!(reply_kind(first), "cursor_travel_superseded");
    send(&fixture, &disable()).expect("disable");
    renderer.join().expect("thread").expect("clean exit");
}

#[test]
fn a_travel_still_in_flight_when_the_renderer_stops_is_answered() {
    let fixture = RegistryFixture::new();
    let fake = FakeLayerCompositor::start(true);
    let renderer = serving(&fixture, &fake, Some("agent-a"), |_| true);
    let travelling = sent(&fixture, &travel(Some("agent-a"), 1500.0, 900.0));
    send(&fixture, &disable()).expect("disable");
    renderer.join().expect("thread").expect("clean exit");
    assert_eq!(reply_kind(travelling), "cursor_renderer_stopped");
}

#[test]
fn a_named_agent_retires_the_unnamed_cursor_and_status_lists_agents() {
    let fixture = RegistryFixture::new();
    let fake = FakeLayerCompositor::start(true);
    let renderer = serving(&fixture, &fake, None, |_| false);
    let status = send(&fixture, &RendererRequest::Status).expect("status");
    assert_eq!(status["status"]["cursors"], 1);
    assert_eq!(status["status"]["agents"], serde_json::json!([]));
    send(&fixture, &travel(Some("agent-a"), 300.0, 400.0)).expect("named present");
    let status = send(&fixture, &RendererRequest::Status).expect("status");
    assert_eq!(status["status"]["cursors"], 1, "{status}");
    assert_eq!(status["status"]["agents"], serde_json::json!(["agent-a"]));
    let destroyed = fake
        .logged()
        .iter()
        .filter(|line| *line == "destroy layer")
        .count();
    assert_eq!(destroyed, 1, "{:?}", fake.logged());
    send(&fixture, &disable()).expect("disable");
    renderer.join().expect("thread").expect("clean exit");
}

fn parked(agent: Option<&str>, x: f64, y: f64) -> RendererRequest {
    match travel(agent, x, y) {
        RendererRequest::Control { control, .. } => RendererRequest::Control {
            control,
            park: true,
        },
        other => other,
    }
}

fn enable(agent: Option<&str>) -> RendererRequest {
    RendererRequest::control(
        CursorOverlayControl::enable(SESSION.into(), CursorOverlayStyle::default())
            .with_agent_id(agent.map(str::to_owned)),
    )
}

#[test]
fn a_parked_cursor_stays_undrawn_when_it_is_redrawn() {
    let fixture = RegistryFixture::new();
    let fake = FakeLayerCompositor::start(true);
    let renderer = serving(&fixture, &fake, Some("agent-a"), |_| true);
    send(&fixture, &travel(Some("agent-a"), 40.0, 40.0)).expect("the travel arrives");
    send(&fixture, &parked(Some("agent-a"), 600.0, 500.0)).expect("the present parks");
    let before = fake.logged().len();
    send(&fixture, &enable(Some("agent-a"))).expect("enable redraws");
    let moved: Vec<String> = fake.logged()[before..]
        .iter()
        .filter(|line| line.starts_with("surface "))
        .cloned()
        .collect();
    assert!(moved.is_empty(), "a parked cursor was drawn: {moved:?}");
    send(&fixture, &disable()).expect("disable");
    renderer.join().expect("thread").expect("clean exit");
}
