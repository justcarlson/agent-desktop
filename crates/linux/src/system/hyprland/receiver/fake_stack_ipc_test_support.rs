//! A bound Hyprland IPC socket served by the test process itself, so a
//! receiver read can be tested against canned replies or silence without
//! a compositor.

use std::io::{Read, Write};
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::thread::JoinHandle;

use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;
use crate::system::process::proc_stat::read_stat;

/// What the fake answers for one command.
pub(crate) struct CannedReply {
    pub(crate) command: &'static str,
    pub(crate) body: &'static str,
}

/// A served socket and the binding that names it.
pub(crate) struct FakeStackIpc {
    pub(crate) binding: HyprlandBinding,
    server: Option<JoinHandle<()>>,
    dir: PathBuf,
}

impl FakeStackIpc {
    /// Answers each command once from `replies`, in any order.
    pub(crate) fn answering(name: &str, replies: Vec<CannedReply>) -> Self {
        let (listener, binding, dir) = bind(name);
        let server = std::thread::spawn(move || {
            for _ in 0..replies.len() {
                let Ok((mut stream, _)) = listener.accept() else {
                    return;
                };
                let mut command = [0_u8; 64];
                let read = stream.read(&mut command).unwrap_or(0);
                let command = String::from_utf8_lossy(&command[..read]).into_owned();
                if let Some(reply) = replies.iter().find(|reply| reply.command == command) {
                    let _ = stream.write_all(reply.body.as_bytes());
                }
            }
        });
        Self {
            binding,
            server: Some(server),
            dir,
        }
    }

    /// Accepts one request and never answers it.
    pub(crate) fn silent(name: &str) -> Self {
        let (listener, binding, dir) = bind(name);
        let server = std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut sink = Vec::new();
                let _ = stream.read_to_end(&mut sink);
            }
        });
        Self {
            binding,
            server: Some(server),
            dir,
        }
    }
}

impl Drop for FakeStackIpc {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
        drop(self.server.take());
    }
}

fn bind(name: &str) -> (UnixListener, HyprlandBinding, PathBuf) {
    let dir = std::env::temp_dir().join(format!("fake-stack-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("socket dir");
    let socket = dir.join(".socket.sock");
    let _ = std::fs::remove_file(&socket);
    let listener = UnixListener::bind(&socket).expect("bind");
    let pid = std::process::id();
    let start_time = read_stat(pid).expect("readable").expect("alive").start_time;
    let binding = HyprlandBinding {
        signature: "test".into(),
        ipc_socket: socket,
        wayland_socket: PathBuf::from("/nonexistent/wayland-1"),
        pid,
        start_time,
    };
    (listener, binding, dir)
}
