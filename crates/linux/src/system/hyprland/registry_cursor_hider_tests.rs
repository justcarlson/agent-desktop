use std::io::{Read, Write};
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};

use agent_desktop_core::{
    AdapterError, Deadline, DisplayInfo, ErrorCode, Rect, ScreenshotTarget, WindowInfo,
};
use serde_json::json;

use super::cursor_hider_for;
use crate::input::hyprland::cursor_guard::with_cursors_hidden;
use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;
use crate::system::hyprland::capture::capture_world::CaptureWorld;
use crate::system::hyprland::capture::hypr_capture::capture_screen;
use crate::system::hyprland::cursor_renderer::fake_renderer::{FakeAnswer, FakeRenderer};
use crate::system::hyprland::cursor_renderer::hold_registry::acknowledged_renderers;
use crate::system::hyprland::cursor_renderer::holder_view::{any_live, live_holders};
use crate::system::hyprland::cursor_renderer::registry_fixture::{RegistryFixture, start_time_of};
use crate::system::hyprland::identity::bound_toplevel::BoundToplevel;
use crate::system::wayland::capture::composition::Composition;
use crate::system::wayland::capture::raw_frame::RawFrame;
use crate::system::wayland::capture::shm_layout::XRGB8888;

const SIGNATURE: &str = "fixture-signature";

/// Serves `j/layers` with `layers` on every connection until the test ends.
fn serve_layers(root: &Path, layers: serde_json::Value) -> PathBuf {
    let socket = root.join(".socket.sock");
    let listener = UnixListener::bind(&socket).expect("bind ipc");
    let reply = json!({"DP-1": {"levels": {"3": layers}}}).to_string();
    std::thread::spawn(move || {
        while let Ok((mut stream, _)) = listener.accept() {
            let mut command = [0_u8; 64];
            let _ = stream.read(&mut command);
            let _ = stream.write_all(reply.as_bytes());
        }
    });
    socket
}

fn binding(ipc_socket: PathBuf) -> HyprlandBinding {
    HyprlandBinding {
        signature: SIGNATURE.into(),
        ipc_socket,
        wayland_socket: PathBuf::from("/nonexistent/wayland-1"),
        pid: std::process::id(),
        start_time: start_time_of(std::process::id()),
    }
}

fn own_cursor_layer(pid: u32) -> serde_json::Value {
    json!([{"namespace": "agent-desktop-cursor", "pid": pid, "x": 0, "y": 0, "w": 32, "h": 32}])
}

fn deadline() -> Deadline {
    Deadline::after(2_000).expect("deadline")
}

fn kind(error: &AdapterError) -> &str {
    error.details.as_ref().expect("details")["kind"]
        .as_str()
        .expect("kind")
}

#[test]
fn headed_input_runs_with_every_renderer_hidden_and_shows_them_after() {
    let fixture = RegistryFixture::new();
    let renderer = FakeRenderer::start(
        &fixture.paths.dir,
        "1111111111111111",
        FakeAnswer::Acknowledge,
    );
    let socket = serve_layers(fixture.root(), own_cursor_layer(std::process::id()));
    let mut hider = cursor_hider_for(&binding(socket), Some(fixture.root()));
    let seen = with_cursors_hidden(hider.as_mut(), deadline(), || {
        Ok((
            renderer.messages(),
            any_live(&fixture.paths.holders),
            acknowledged_renderers(),
        ))
    })
    .expect("sent");
    assert_eq!(seen.0, ["hide"]);
    assert!(
        seen.1,
        "a renderer that starts during the work finds a live holder"
    );
    assert_eq!(seen.2, [std::process::id()]);
    assert_eq!(renderer.messages(), ["hide", "show"]);
    assert!(!any_live(&fixture.paths.holders));
}

#[test]
fn a_withheld_hide_acknowledgement_refuses_before_input() {
    let fixture = RegistryFixture::new();
    let _renderer =
        FakeRenderer::start(&fixture.paths.dir, "2222222222222222", FakeAnswer::Withhold);
    let socket = serve_layers(fixture.root(), json!([]));
    let mut hider = cursor_hider_for(&binding(socket), Some(fixture.root()));
    let mut sent = false;
    let error = with_cursors_hidden(hider.as_mut(), deadline(), || {
        sent = true;
        Ok(())
    })
    .expect_err("withheld");
    assert!(!sent);
    assert_eq!(error.code, ErrorCode::Timeout);
    assert_eq!(kind(&error), "cursor_hide_timeout");
    assert_eq!(
        error.details.as_ref().expect("details")["physical_delivery_started"],
        false
    );
}

#[test]
fn a_foreign_layer_with_the_cursor_namespace_refuses_before_input() {
    for runtime in [true, false] {
        let fixture = RegistryFixture::new();
        let socket = serve_layers(fixture.root(), own_cursor_layer(1));
        let mut hider = cursor_hider_for(&binding(socket), runtime.then(|| fixture.root()));
        let mut sent = false;
        let error = with_cursors_hidden(hider.as_mut(), deadline(), || {
            sent = true;
            Ok(())
        })
        .expect_err("foreign layer");
        assert!(!sent);
        assert_eq!(error.code, ErrorCode::ActionFailed);
        assert_eq!(kind(&error), "cursor_unknown_overlay");
    }
}

#[test]
fn show_from_one_hider_keeps_another_hiders_suppression() {
    let fixture = RegistryFixture::new();
    let renderer = FakeRenderer::start(
        &fixture.paths.dir,
        "3333333333333333",
        FakeAnswer::Acknowledge,
    );
    let socket = serve_layers(fixture.root(), json!([]));
    let mut first = cursor_hider_for(&binding(socket.clone()), Some(fixture.root()));
    let mut second = cursor_hider_for(&binding(socket), Some(fixture.root()));
    first.hide(deadline()).expect("first hide");
    second.hide(deadline()).expect("second hide");
    first.show();
    assert_eq!(live_holders(&fixture.paths.holders).len(), 1);
    second.show();
    assert!(!any_live(&fixture.paths.holders));
    assert_eq!(renderer.messages(), ["hide", "hide", "show", "show"]);
}

/// A one-display world whose frame records what the renderer had been told
/// and whether a holder was live when the frame was taken.
struct ScreenWorld<'a> {
    renderer: &'a FakeRenderer,
    holders: PathBuf,
    seen: Option<(Vec<String>, bool)>,
}

impl CaptureWorld for ScreenWorld<'_> {
    fn enforced(&mut self) -> Result<bool, AdapterError> {
        Ok(false)
    }

    fn bind(&mut self, _win: &WindowInfo) -> Result<BoundToplevel, AdapterError> {
        Err(AdapterError::internal("screen only"))
    }

    fn composition(&mut self, _bound: &BoundToplevel) -> Result<Composition, AdapterError> {
        Err(AdapterError::internal("screen only"))
    }

    fn capture_toplevel(
        &mut self,
        _id: &str,
        _composition: Composition,
    ) -> Result<RawFrame, AdapterError> {
        Err(AdapterError::internal("screen only"))
    }

    fn still_bound(
        &mut self,
        _win: &WindowInfo,
        _bound: &BoundToplevel,
    ) -> Result<bool, AdapterError> {
        Err(AdapterError::internal("screen only"))
    }

    fn displays(&mut self) -> Result<Vec<DisplayInfo>, AdapterError> {
        Ok(vec![DisplayInfo {
            id: "DP-1".into(),
            bounds: Rect {
                x: 0.0,
                y: 0.0,
                width: 4.0,
                height: 4.0,
            },
            is_primary: true,
            scale: 1.0,
        }])
    }

    fn capture_output(&mut self, _name: &str) -> Result<RawFrame, AdapterError> {
        self.seen = Some((self.renderer.messages(), any_live(&self.holders)));
        Ok(RawFrame {
            data: vec![0x40; 64],
            format: XRGB8888,
            width: 4,
            height: 4,
            stride: 16,
            inverted: false,
        })
    }
}

#[test]
fn a_screen_capture_is_taken_while_every_drawing_renderer_is_hidden() {
    let fixture = RegistryFixture::new();
    let renderer = FakeRenderer::start(
        &fixture.paths.dir,
        "4444444444444444",
        FakeAnswer::Acknowledge,
    );
    let socket = serve_layers(fixture.root(), own_cursor_layer(std::process::id()));
    let mut hider = cursor_hider_for(&binding(socket), Some(fixture.root()));
    let mut world = ScreenWorld {
        renderer: &renderer,
        holders: fixture.paths.holders.clone(),
        seen: None,
    };
    capture_screen(
        &mut world,
        hider.as_mut(),
        &ScreenshotTarget::FullScreen,
        deadline(),
    )
    .expect("captured");
    assert_eq!(world.seen, Some((vec!["hide".to_owned()], true)));
    assert_eq!(renderer.messages(), ["hide", "show"]);
}
