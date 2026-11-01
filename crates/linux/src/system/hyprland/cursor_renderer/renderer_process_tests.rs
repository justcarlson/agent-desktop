use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use agent_desktop_core::{
    AdapterError, CursorOverlayConfig, CursorOverlayControl, CursorOverlayInstruction,
    CursorOverlayStyle, Point,
};
use serde_json::Value;

use super::RendererProcess;
use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;
use crate::system::hyprland::cursor_renderer::fake_layer_compositor::FakeLayerCompositor;
use crate::system::hyprland::cursor_renderer::record_read::{RecordRead, read_record};
use crate::system::hyprland::cursor_renderer::registry_fixture::{RegistryFixture, record_for};
use crate::system::hyprland::cursor_renderer::renderer_request::RendererRequest;
use crate::system::hyprland::cursor_renderer::renderer_target::RendererTarget;
use crate::system::hyprland::cursor_renderer::renderer_wire::exchange;

const SESSION: &str = "fixture-session";

fn soon() -> Instant {
    Instant::now() + Duration::from_secs(3)
}

fn target(fixture: &RegistryFixture, fake: &FakeLayerCompositor) -> RendererTarget {
    std::fs::create_dir_all(fixture.root().join("hypr").join("fixture-signature"))
        .expect("compositor dir");
    RendererTarget {
        binding: HyprlandBinding {
            signature: "fixture-signature".into(),
            ipc_socket: fixture.root().join(".socket.sock"),
            wayland_socket: fake.socket.clone(),
            pid: 1,
            start_time: 1,
        },
        runtime_dir: fixture.root().to_path_buf(),
    }
}

fn serving(
    fixture: &RegistryFixture,
    fake: &FakeLayerCompositor,
) -> JoinHandle<Result<(), AdapterError>> {
    let enable = CursorOverlayControl::enable(SESSION.into(), CursorOverlayStyle::default());
    serving_from(fixture, fake, &enable, |_| false)
}

fn serving_from(
    fixture: &RegistryFixture,
    fake: &FakeLayerCompositor,
    first: &CursorOverlayControl,
    multi_agent: fn(&str) -> bool,
) -> JoinHandle<Result<(), AdapterError>> {
    let process = RendererProcess::start_over(
        fake.connect(),
        fixture.paths.clone(),
        &target(fixture, fake),
        first,
    )
    .expect("renderer starts")
    .with_wanted(|_| true)
    .with_multi_agent(multi_agent);
    std::thread::spawn(move || process.serve())
}

fn send(fixture: &RegistryFixture, request: &RendererRequest) -> Result<Value, AdapterError> {
    exchange(&fixture.paths.socket, request, soon()).map(|reply| reply.expect("renderer listens"))
}

fn kind_of(error: AdapterError) -> Value {
    error.details.expect("details")["kind"].clone()
}

fn travel_to(x: f64, y: f64) -> RendererRequest {
    let config = CursorOverlayConfig::enabled(None, 6).expect("config");
    let instruction = CursorOverlayInstruction::new(Point { x, y }, &config, true).expect("instr");
    RendererRequest::control(CursorOverlayControl::present(SESSION.into(), instruction))
}

fn disabled(fixture: &RegistryFixture, renderer: JoinHandle<Result<(), AdapterError>>) {
    let disable = RendererRequest::control(CursorOverlayControl::disable(SESSION.into()));
    send(fixture, &disable).expect("disable");
    renderer
        .join()
        .expect("renderer thread")
        .expect("clean exit");
}

#[test]
fn the_renderer_serves_status_present_and_disable_then_removes_its_own_files() {
    let fixture = RegistryFixture::new();
    let fake = FakeLayerCompositor::start(true);
    let renderer = serving(&fixture, &fake);

    let status = send(&fixture, &RendererRequest::Status).expect("status");
    assert_eq!(status["status"]["pid"], std::process::id());
    assert_eq!(status["status"]["cursors"], 1);
    assert_eq!(status["status"]["holders"], 0);
    assert_eq!(status["status"]["protocol"], "wayland-v1");
    assert!(matches!(
        read_record(&fixture.paths.record),
        RecordRead::Present(_)
    ));

    send(&fixture, &travel_to(300.0, 400.0)).expect("present travels and arrives");
    assert!(
        fake.logged()
            .iter()
            .any(|line| line == "margin 399 0 0 299"),
        "{:?}",
        fake.logged()
    );

    let disable = RendererRequest::control(CursorOverlayControl::disable(SESSION.into()));
    send(&fixture, &disable).expect("disable answers");
    renderer
        .join()
        .expect("renderer thread")
        .expect("clean exit");
    assert_eq!(read_record(&fixture.paths.record), RecordRead::Absent);
    assert!(!fixture.paths.socket.exists());
    assert!(fake.logged().iter().any(|line| line == "destroy layer"));
}

#[test]
fn a_hide_for_no_live_holder_and_another_sessions_control_are_refused_by_kind() {
    let fixture = RegistryFixture::new();
    let fake = FakeLayerCompositor::start(true);
    let renderer = serving(&fixture, &fake);
    let hide = RendererRequest::Hide {
        holder: "n1".into(),
        expiry_ms: 500,
    };
    let refused = send(&fixture, &hide).expect_err("no such holder");
    assert_eq!(kind_of(refused), "cursor_holder_not_live");
    let other = RendererRequest::control(CursorOverlayControl::hide("other-session".into()));
    let refused = send(&fixture, &other).expect_err("another session");
    assert_eq!(kind_of(refused), "cursor_renderer_session_mismatch");
    let disable = RendererRequest::control(CursorOverlayControl::disable(SESSION.into()));
    send(&fixture, &disable).expect("disable");
    renderer
        .join()
        .expect("renderer thread")
        .expect("clean exit");
}

#[test]
fn a_renderer_whose_record_names_a_newer_generation_exits_and_leaves_it() {
    let fixture = RegistryFixture::new();
    let fake = FakeLayerCompositor::start(true);
    let renderer = serving(&fixture, &fake);
    record_for(std::process::id(), None, "newer")
        .write_atomic(&fixture.paths.record)
        .expect("newer record");
    renderer
        .join()
        .expect("renderer thread")
        .expect("clean exit");
    match read_record(&fixture.paths.record) {
        RecordRead::Present(record) => assert_eq!(record.owner.generation, "newer"),
        other => panic!("the newer generation's record was removed: {other:?}"),
    }
}

#[test]
fn a_renderer_exits_when_its_compositor_instance_goes() {
    let fixture = RegistryFixture::new();
    let fake = FakeLayerCompositor::start(true);
    let renderer = serving(&fixture, &fake);
    std::fs::remove_dir_all(fixture.root().join("hypr")).expect("remove compositor dir");
    renderer
        .join()
        .expect("renderer thread")
        .expect("clean exit");
    assert_eq!(read_record(&fixture.paths.record), RecordRead::Absent);
}

fn margins(fake: &FakeLayerCompositor) -> Vec<String> {
    fake.logged()
        .into_iter()
        .filter(|line| line.starts_with("margin "))
        .collect()
}

#[test]
fn the_cursor_shows_before_any_present_and_a_later_enable_does_not_move_it() {
    let fixture = RegistryFixture::new();
    let fake = FakeLayerCompositor::start(true);
    let renderer = serving(&fixture, &fake);
    send(&fixture, &RendererRequest::Status).expect("status");
    assert!(
        fake.logged()
            .iter()
            .any(|line| line == "layer 3 namespace agent-desktop-cursor output 0"),
        "no cursor layer after the first enable: {:?}",
        fake.logged()
    );
    assert_eq!(margins(&fake), ["margin -1 0 0 -1"]);

    send(&fixture, &travel_to(300.0, 400.0)).expect("present");
    let placed = margins(&fake);
    let enable = CursorOverlayControl::enable(SESSION.into(), CursorOverlayStyle::default());
    send(&fixture, &RendererRequest::control(enable)).expect("second enable");
    assert_eq!(margins(&fake), placed, "a later enable moved the cursor");

    let disable = RendererRequest::control(CursorOverlayControl::disable(SESSION.into()));
    send(&fixture, &disable).expect("disable");
    renderer
        .join()
        .expect("renderer thread")
        .expect("clean exit");
}

fn agent_travel(agent: Option<&str>, x: f64, y: f64, style: CursorOverlayStyle) -> RendererRequest {
    let config = CursorOverlayConfig::enabled(None, 6).expect("config");
    let instruction = CursorOverlayInstruction::new(Point { x, y }, &config, true).expect("instr");
    let control = CursorOverlayControl::present_with_style(SESSION.into(), instruction, style)
        .with_agent_id(agent.map(str::to_owned));
    RendererRequest::control(control)
}

fn agent_serving(
    fixture: &RegistryFixture,
    fake: &FakeLayerCompositor,
) -> JoinHandle<Result<(), AdapterError>> {
    let enable = CursorOverlayControl::enable(SESSION.into(), CursorOverlayStyle::default())
        .with_agent_id(Some("agent-a".into()));
    serving_from(fixture, fake, &enable, |_| true)
}

fn layers_made(fake: &FakeLayerCompositor) -> usize {
    fake.logged()
        .iter()
        .filter(|line| line.starts_with("layer 3 namespace agent-desktop-cursor"))
        .count()
}

fn surface_at(fake: &FakeLayerCompositor, left: i32, top: i32) -> Vec<String> {
    let place = format!(" at {left} {top}");
    fake.logged()
        .into_iter()
        .filter(|line| line.starts_with("surface ") && line.ends_with(&place))
        .filter_map(|line| line.split(' ').nth(1).map(str::to_owned))
        .collect()
}

#[test]
fn a_second_agents_present_draws_a_second_cursor_in_the_same_renderer() {
    let fixture = RegistryFixture::new();
    let fake = FakeLayerCompositor::start(true);
    let renderer = agent_serving(&fixture, &fake);
    let plain = CursorOverlayStyle::default();
    send(
        &fixture,
        &agent_travel(Some("agent-a"), 300.0, 400.0, plain.clone()),
    )
    .expect("a");
    send(
        &fixture,
        &agent_travel(Some("agent-b"), 500.0, 200.0, plain.clone()),
    )
    .expect("b");
    send(
        &fixture,
        &agent_travel(Some("agent-a"), 310.0, 410.0, plain),
    )
    .expect("a again");

    let status = send(&fixture, &RendererRequest::Status).expect("status");
    assert_eq!(status["status"]["cursors"], 2, "{status}");
    assert_eq!(status["status"]["pid"], std::process::id());
    assert_eq!(layers_made(&fake), 2, "{:?}", fake.logged());
    let first = surface_at(&fake, 309, 409);
    let second = surface_at(&fake, 499, 199);
    assert!(
        !first.is_empty() && !second.is_empty(),
        "{:?}",
        fake.logged()
    );
    assert_ne!(first, second, "both agents drew on one surface");
    assert!(
        !fake.logged().iter().any(|line| line == "destroy layer"),
        "an agent's present removed another agent's cursor: {:?}",
        fake.logged()
    );
    disabled(&fixture, renderer);
}

#[test]
fn a_multi_agent_renderer_refuses_a_control_with_no_agent_id() {
    let fixture = RegistryFixture::new();
    let fake = FakeLayerCompositor::start(true);
    let renderer = agent_serving(&fixture, &fake);
    let made = layers_made(&fake);
    let anonymous = agent_travel(None, 300.0, 400.0, CursorOverlayStyle::default());
    let refused = send(&fixture, &anonymous).expect_err("no agent id");
    assert_eq!(kind_of(refused), "cursor_agent_id_required");
    let hide = RendererRequest::control(CursorOverlayControl::hide(SESSION.into()));
    let refused = send(&fixture, &hide).expect_err("no agent id on hide");
    assert_eq!(kind_of(refused), "cursor_agent_id_required");
    assert!(
        surface_at(&fake, 299, 399).is_empty(),
        "{:?}",
        fake.logged()
    );
    assert_eq!(layers_made(&fake), made);
    let status = send(&fixture, &RendererRequest::Status).expect("status");
    assert_eq!(status["status"]["cursors"], 1, "{status}");
    disabled(&fixture, renderer);
}

#[test]
fn a_profile_changes_how_an_agents_cursor_looks_not_where_it_lands() {
    let fixture = RegistryFixture::new();
    let fake = FakeLayerCompositor::start(true);
    let renderer = agent_serving(&fixture, &fake);
    let mut large = CursorOverlayStyle::default();
    large.set_size(2.0);
    send(
        &fixture,
        &agent_travel(Some("agent-a"), 300.0, 400.0, CursorOverlayStyle::default()),
    )
    .expect("a");
    send(
        &fixture,
        &agent_travel(Some("agent-b"), 300.0, 400.0, large),
    )
    .expect("b");
    let sizes: Vec<String> = fake
        .logged()
        .into_iter()
        .filter(|line| line.starts_with("size "))
        .collect();
    assert_eq!(sizes, ["size 24 32", "size 46 62"], "{:?}", fake.logged());
    let mut landed = surface_at(&fake, 299, 399);
    landed.dedup();
    assert_eq!(
        landed.len(),
        2,
        "both profiles land on the target: {landed:?}"
    );
    assert!(
        !fake.logged().iter().any(|line| line == "destroy layer"),
        "one agent's profile rebuilt another agent's cursor: {:?}",
        fake.logged()
    );
    disabled(&fixture, renderer);
}

#[test]
fn two_agents_travel_at_the_same_time() {
    let fixture = RegistryFixture::new();
    let fake = FakeLayerCompositor::start(true);
    let renderer = agent_serving(&fixture, &fake);
    let plain = CursorOverlayStyle::default();
    send(
        &fixture,
        &agent_travel(Some("agent-b"), 10.0, 10.0, plain.clone()),
    )
    .expect("place b");
    let socket = fixture.paths.socket.clone();
    let far = |agent: &'static str, x: f64, y: f64| {
        let request = agent_travel(Some(agent), x, y, CursorOverlayStyle::default());
        let socket = socket.clone();
        std::thread::spawn(move || exchange(&socket, &request, soon()))
    };
    let a = far("agent-a", 1500.0, 900.0);
    let b = far("agent-b", 1200.0, 50.0);
    a.join().expect("a thread").expect("a arrives");
    b.join().expect("b thread").expect("b arrives");
    let log = fake.logged();
    let a_surface = surface_at(&fake, 1499, 899).pop().expect("a landed");
    let b_surface = surface_at(&fake, 1199, 49).pop().expect("b landed");
    assert_ne!(a_surface, b_surface, "both agents moved one cursor");
    let moves = |surface: &str| -> Vec<usize> {
        let prefix = format!("surface {surface} at ");
        log.iter()
            .enumerate()
            .filter(|(_, line)| line.starts_with(&prefix))
            .map(|(index, _)| index)
            .collect()
    };
    let (a_moves, b_moves) = (moves(&a_surface), moves(&b_surface));
    let a_last = *a_moves.last().expect("a moved");
    let b_last = *b_moves.last().expect("b moved");
    let a_during_b = a_moves.iter().filter(|index| **index < b_last).count();
    let b_during_a = b_moves.iter().filter(|index| **index < a_last).count();
    assert!(
        a_during_b > 2 && b_during_a > 2,
        "the travels did not overlap: a {a_moves:?} b {b_moves:?}"
    );
    disabled(&fixture, renderer);
}
