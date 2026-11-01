use std::os::unix::net::UnixListener;
use std::sync::Mutex;
use std::thread::JoinHandle;

use agent_desktop_core::{AdapterError, CursorOverlayControl, CursorOverlayStyle};
use serde_json::json;

use super::{take_outcome, update_routed};
use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;
use crate::system::hyprland::cursor_renderer::registry_fixture::RegistryFixture;
use crate::system::hyprland::cursor_renderer::renderer_launcher::RendererLauncher;
use crate::system::hyprland::cursor_renderer::renderer_paths::RendererPaths;
use crate::system::hyprland::cursor_renderer::renderer_pidfd::RendererPidfd;
use crate::system::hyprland::cursor_renderer::renderer_request::RendererRequest;
use crate::system::hyprland::cursor_renderer::renderer_target::RendererTarget;
use crate::system::hyprland::cursor_renderer::renderer_wire::{
    ok_reply, read_request, write_reply,
};

/// Starts a stand-in renderer that acknowledges every request it reads.
#[derive(Default)]
struct AnsweringLauncher {
    served: Mutex<Vec<JoinHandle<Vec<String>>>>,
}

impl RendererLauncher for AnsweringLauncher {
    fn session_active(&self, _: &CursorOverlayControl) -> bool {
        true
    }

    fn launch(
        &self,
        _: &RendererTarget,
        paths: &RendererPaths,
        _: &CursorOverlayControl,
    ) -> Result<Option<RendererPidfd>, AdapterError> {
        let listener = UnixListener::bind(&paths.socket).expect("bind");
        let served = std::thread::spawn(move || {
            let mut messages = Vec::new();
            for stream in listener.incoming().take(2) {
                let Ok(stream) = stream else { break };
                if let Ok(request) = read_request(&stream) {
                    messages.push(request.message().to_owned());
                    write_reply(&stream, &ok_reply(json!({})));
                }
            }
            messages
        });
        self.served.lock().expect("served").push(served);
        Ok(None)
    }
}

fn target(fixture: &RegistryFixture) -> RendererTarget {
    RendererTarget {
        binding: HyprlandBinding {
            signature: "fixture-signature".into(),
            ipc_socket: fixture.root().join(".socket.sock"),
            wayland_socket: fixture.root().join("wayland-1"),
            pid: 1,
            start_time: 1,
        },
        runtime_dir: fixture.root().to_path_buf(),
    }
}

fn enable() -> CursorOverlayControl {
    CursorOverlayControl::enable("fixture-session".into(), CursorOverlayStyle::default())
}

#[test]
fn enable_on_the_hyprland_route_starts_a_renderer_and_is_rendered() {
    let fixture = RegistryFixture::new();
    let launcher = AnsweringLauncher::default();
    update_routed(&enable(), Some(target(&fixture)), &launcher)
        .expect("enable on the Hyprland route renders");
    let served = launcher
        .served
        .lock()
        .expect("served")
        .pop()
        .expect("one launch");
    drop(std::os::unix::net::UnixStream::connect(
        &fixture.paths.socket,
    ));
    assert!(
        served
            .join()
            .expect("renderer")
            .contains(&"enable".to_owned())
    );
}

#[test]
fn a_session_off_hyprland_is_still_not_rendered() {
    let error =
        update_routed(&enable(), None, &AnsweringLauncher::default()).expect_err("not rendered");
    assert_eq!(error.details.expect("details")["rendered"], false);
}

#[test]
fn a_transient_hide_with_no_renderer_starts_nothing() {
    let fixture = RegistryFixture::new();
    let launcher = AnsweringLauncher::default();
    let hide = CursorOverlayControl::hide("fixture-session".into());
    update_routed(&hide, Some(target(&fixture)), &launcher).expect("hide");
    assert!(launcher.served.lock().expect("served").is_empty());
}

#[test]
fn disable_with_nothing_running_reports_not_running_once() {
    let fixture = RegistryFixture::new();
    let disable = CursorOverlayControl::disable("fixture-session".into());
    update_routed(
        &disable,
        Some(target(&fixture)),
        &AnsweringLauncher::default(),
    )
    .expect("disable");
    assert_eq!(take_outcome("fixture-session"), Some("not_running"));
    assert_eq!(take_outcome("fixture-session"), None);
}

#[test]
fn a_renderer_that_answers_gets_the_control_without_a_start() {
    let fixture = RegistryFixture::new();
    let listener = UnixListener::bind(&fixture.paths.socket).expect("bind");
    let renderer = std::thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept");
        let request = read_request(&stream).expect("request");
        write_reply(&stream, &ok_reply(json!({})));
        request
    });
    let launcher = AnsweringLauncher::default();
    update_routed(&enable(), Some(target(&fixture)), &launcher).expect("enable");
    assert!(matches!(
        renderer.join().expect("renderer"),
        RendererRequest::Control { .. }
    ));
    assert!(launcher.served.lock().expect("served").is_empty());
}
