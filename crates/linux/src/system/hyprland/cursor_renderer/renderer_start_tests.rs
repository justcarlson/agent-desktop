use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::sync::{Arc, Barrier, Mutex};
use std::time::{Duration, Instant};

use agent_desktop_core::{AdapterError, CursorOverlayControl, CursorOverlayStyle};

use super::{StartRequest, ensure_running};
use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;
use crate::system::hyprland::cursor_renderer::exe_identity::ExeIdentity;
use crate::system::hyprland::cursor_renderer::record_read::{RecordRead, read_record};
use crate::system::hyprland::cursor_renderer::registry_fixture::{
    RegistryFixture, finish, is_running, record_for, term_ignoring_sleeper,
};
use crate::system::hyprland::cursor_renderer::renderer_launcher::RendererLauncher;
use crate::system::hyprland::cursor_renderer::renderer_paths::RendererPaths;
use crate::system::hyprland::cursor_renderer::renderer_target::RendererTarget;

/// Stands in for a renderer: it starts listening a little after launch.
#[derive(Default)]
struct FakeLauncher {
    listeners: Mutex<Vec<UnixListener>>,
    launches: Mutex<u32>,
}

impl RendererLauncher for FakeLauncher {
    fn session_active(&self, _: &CursorOverlayControl) -> bool {
        true
    }

    fn launch(
        &self,
        _: &RendererTarget,
        paths: &RendererPaths,
        _: &CursorOverlayControl,
    ) -> Result<(), AdapterError> {
        *self.launches.lock().expect("count") += 1;
        std::thread::sleep(Duration::from_millis(50));
        let listener = UnixListener::bind(&paths.socket).expect("fake renderer binds");
        self.listeners.lock().expect("listeners").push(listener);
        Ok(())
    }
}

impl FakeLauncher {
    fn launches(&self) -> u32 {
        *self.launches.lock().expect("count")
    }
}

pub(crate) fn target() -> RendererTarget {
    RendererTarget {
        binding: HyprlandBinding {
            signature: "fixture-signature".into(),
            ipc_socket: PathBuf::from("/nonexistent/.socket.sock"),
            wayland_socket: PathBuf::from("/nonexistent/wayland-1"),
            pid: 1,
            start_time: 1,
        },
        runtime_dir: PathBuf::from("/nonexistent"),
    }
}

fn enable() -> CursorOverlayControl {
    CursorOverlayControl::enable("fixture-session".into(), CursorOverlayStyle::default())
}

fn start(paths: &RendererPaths, launcher: &FakeLauncher) -> Result<(), AdapterError> {
    let target = target();
    let first = enable();
    let request = StartRequest {
        paths,
        target: &target,
        first: &first,
        deadline: Instant::now() + Duration::from_secs(3),
    };
    ensure_running(&request, launcher, ExeIdentity::current())
}

#[test]
fn two_concurrent_starters_give_one_renderer() {
    let fixture = RegistryFixture::new();
    let launcher = Arc::new(FakeLauncher::default());
    let barrier = Arc::new(Barrier::new(2));
    let starters: Vec<_> = (0..2)
        .map(|_| {
            let paths = fixture.paths.clone();
            let launcher = Arc::clone(&launcher);
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                start(&paths, &launcher)
            })
        })
        .collect();
    for starter in starters {
        starter.join().expect("starter").expect("started");
    }
    assert_eq!(launcher.launches(), 1);
}

#[test]
fn an_incompatible_live_record_keeps_its_files_and_gets_no_signal_and_no_duplicate() {
    let fixture = RegistryFixture::new();
    let other_build = term_ignoring_sleeper();
    std::thread::sleep(Duration::from_millis(100));
    record_for(other_build.id(), None, "other")
        .write_atomic(&fixture.paths.record)
        .expect("write");
    std::fs::write(&fixture.paths.socket, b"").expect("socket file");
    let launcher = FakeLauncher::default();
    let error = start(&fixture.paths, &launcher).expect_err("refused");
    let alive = is_running(&other_build);
    finish(other_build);
    assert_eq!(
        error.details.expect("details")["kind"],
        "cursor_renderer_incompatible"
    );
    assert_eq!(launcher.launches(), 0);
    assert!(alive, "another build's renderer was signalled");
    assert!(fixture.paths.socket.exists(), "its socket was removed");
    assert!(matches!(
        read_record(&fixture.paths.record),
        RecordRead::Present(_)
    ));
}

#[test]
fn a_stale_record_is_cleared_and_one_renderer_starts() {
    let fixture = RegistryFixture::new();
    let mut gone = std::process::Command::new("true").spawn().expect("spawn");
    let pid = gone.id();
    let start_time = crate::system::hyprland::cursor_renderer::registry_fixture::start_time_of(pid);
    gone.wait().expect("reaped");
    record_for(pid, Some(start_time), "stale")
        .write_atomic(&fixture.paths.record)
        .expect("write");
    std::fs::write(&fixture.paths.socket, b"").expect("leftover socket");
    let launcher = FakeLauncher::default();
    start(&fixture.paths, &launcher).expect("started");
    assert_eq!(launcher.launches(), 1);
    assert_eq!(read_record(&fixture.paths.record), RecordRead::Absent);
}

#[test]
fn a_renderer_already_listening_is_not_started_again() {
    let fixture = RegistryFixture::new();
    let _listening = UnixListener::bind(&fixture.paths.socket).expect("bind");
    let launcher = FakeLauncher::default();
    start(&fixture.paths, &launcher).expect("answers");
    assert_eq!(launcher.launches(), 0);
}
