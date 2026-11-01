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
    let process = RendererProcess::start_over(
        fake.connect(),
        fixture.paths.clone(),
        &target(fixture, fake),
        &enable,
    )
    .expect("renderer starts")
    .with_wanted(|_| true);
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
fn hide_holders_and_another_sessions_control_are_refused_by_kind() {
    let fixture = RegistryFixture::new();
    let fake = FakeLayerCompositor::start(true);
    let renderer = serving(&fixture, &fake);
    let hide = RendererRequest::Hide {
        holder: "n1".into(),
        expiry_ms: 500,
    };
    let refused = send(&fixture, &hide).expect_err("holders are not served");
    assert_eq!(kind_of(refused), "cursor_holders_not_supported");
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
