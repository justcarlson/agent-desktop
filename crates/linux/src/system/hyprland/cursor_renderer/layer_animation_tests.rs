use std::io::{Read, Write};
use std::os::unix::net::UnixListener;
use std::path::Path;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use agent_desktop_core::{CursorOverlayControl, CursorOverlayStyle};

use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;
use crate::system::hyprland::cursor_renderer::fake_layer_compositor::FakeLayerCompositor;
use crate::system::hyprland::cursor_renderer::registry_fixture::RegistryFixture;
use crate::system::hyprland::cursor_renderer::renderer_process::RendererProcess;
use crate::system::hyprland::cursor_renderer::renderer_target::RendererTarget;
use crate::system::process::proc_stat::read_stat;

const RULE: &str = "eval hl.layer_rule({ name = \"agent-desktop-cursor-no-anim\", \
                    match = { namespace = \"^agent-desktop-cursor$\" }, no_anim = true })";

fn record_commands(socket: &Path) -> Arc<Mutex<Vec<String>>> {
    let _ = std::fs::remove_file(socket);
    let listener = UnixListener::bind(socket).expect("bind ipc");
    let commands = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::clone(&commands);
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { return };
            let mut command = vec![0_u8; 4096];
            let read = stream.read(&mut command).unwrap_or(0);
            let text = String::from_utf8_lossy(&command[..read]).into_owned();
            let reply: &[u8] = if text.starts_with("eval ") {
                b"ok"
            } else {
                b"[]"
            };
            seen.lock().expect("commands").push(text);
            let _ = stream.write_all(reply);
        }
    });
    commands
}

fn own_target(fixture: &RegistryFixture, fake: &FakeLayerCompositor) -> RendererTarget {
    std::fs::create_dir_all(fixture.root().join("hypr").join("fixture-signature"))
        .expect("compositor dir");
    let start_time = read_stat(std::process::id())
        .expect("readable")
        .expect("alive")
        .start_time;
    RendererTarget {
        binding: HyprlandBinding {
            signature: "fixture-signature".into(),
            ipc_socket: fixture.root().join(".socket.sock"),
            wayland_socket: fake.socket.clone(),
            pid: std::process::id(),
            start_time,
        },
        runtime_dir: fixture.root().to_path_buf(),
    }
}

#[test]
fn a_starting_renderer_asks_its_compositor_not_to_animate_cursor_layers() {
    let fixture = RegistryFixture::new();
    let fake = FakeLayerCompositor::start(true);
    let target = own_target(&fixture, &fake);
    let commands = record_commands(&target.binding.ipc_socket);
    let enable =
        CursorOverlayControl::enable("fixture-session".into(), CursorOverlayStyle::default());

    let process =
        RendererProcess::start_over(fake.connect(), fixture.paths.clone(), &target, &enable)
            .expect("renderer starts");

    let seen = commands.lock().expect("commands").clone();
    assert!(seen.iter().any(|command| command == RULE), "{seen:?}");
    drop(process);
}

fn announce_reload_on_signal(socket: &Path) -> mpsc::Sender<()> {
    let _ = std::fs::remove_file(socket);
    let listener = UnixListener::bind(socket).expect("bind events");
    let (signal, signalled) = mpsc::channel::<()>();
    std::thread::spawn(move || {
        let Ok((mut stream, _)) = listener.accept() else {
            return;
        };
        while signalled.recv().is_ok() {
            let _ = stream.write_all(b"activewindow>>foot,shell\nconfigreloaded>>\n");
        }
    });
    signal
}

fn rules_sent(commands: &Arc<Mutex<Vec<String>>>) -> usize {
    let seen = commands.lock().expect("commands");
    seen.iter().filter(|command| *command == RULE).count()
}

#[test]
fn a_config_reload_makes_a_running_renderer_ask_again_not_to_animate_cursor_layers() {
    let fixture = RegistryFixture::new();
    let fake = FakeLayerCompositor::start(true);
    let target = own_target(&fixture, &fake);
    let commands = record_commands(&target.binding.ipc_socket);
    let reload = announce_reload_on_signal(&fixture.root().join(".socket2.sock"));
    let enable =
        CursorOverlayControl::enable("fixture-session".into(), CursorOverlayStyle::default());
    let process =
        RendererProcess::start_over(fake.connect(), fixture.paths.clone(), &target, &enable)
            .expect("renderer starts")
            .with_wanted(|_| true);
    let renderer = std::thread::spawn(move || process.serve());
    assert_eq!(rules_sent(&commands), 1);

    reload.send(()).expect("announce reload");
    let deadline = Instant::now() + Duration::from_secs(3);
    while rules_sent(&commands) < 2 && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }

    let sent = rules_sent(&commands);
    std::fs::remove_dir_all(fixture.root().join("hypr")).expect("remove compositor dir");
    renderer
        .join()
        .expect("renderer thread")
        .expect("clean exit");
    assert_eq!(sent, 2, "{:?}", commands.lock().expect("commands"));
}
