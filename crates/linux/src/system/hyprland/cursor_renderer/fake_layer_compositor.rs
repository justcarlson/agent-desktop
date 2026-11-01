//! A scripted compositor for the renderer's unit tests. It advertises the
//! drawing globals beside a seat, an activation global and outputs, logs
//! every bind and every request that shapes the cursor surface, sends a
//! configure on a layer surface's first commit, and answers frame callbacks
//! unless told to stay dark, as a compositor with every output off does.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// The globals the fake advertises.
pub(crate) const ADVERTISED: [(&str, u32); 6] = [
    ("wl_compositor", 6),
    ("wl_shm", 1),
    ("zwlr_layer_shell_v1", 4),
    ("wl_seat", 9),
    ("xdg_activation_v1", 1),
    ("wl_output", 4),
];

/// A listening fake compositor; its socket directory goes when it drops.
pub(crate) struct FakeLayerCompositor {
    pub(crate) socket: PathBuf,
    pub(crate) log: Arc<Mutex<Vec<String>>>,
    dir: PathBuf,
}

impl FakeLayerCompositor {
    /// Serves one connection; `frames` says whether frame callbacks fire.
    pub(crate) fn start(frames: bool) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "agent-desktop-fake-layer-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).expect("socket directory");
        let socket = dir.join("wayland-fake");
        let listener = UnixListener::bind(&socket).expect("bind fake compositor");
        let log = Arc::new(Mutex::new(Vec::new()));
        let served = Arc::clone(&log);
        std::thread::spawn(move || {
            if let Ok((stream, _)) = listener.accept() {
                serve(stream, frames, &served);
            }
        });
        Self { socket, log, dir }
    }

    /// Connects a client stream to the fake.
    pub(crate) fn connect(&self) -> UnixStream {
        UnixStream::connect(&self.socket).expect("connect fake compositor")
    }

    pub(crate) fn logged(&self) -> Vec<String> {
        self.log.lock().expect("log").clone()
    }
}

impl Drop for FakeLayerCompositor {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Display,
    Registry,
    Compositor,
    Surface,
    Region(bool),
    LayerShell,
    Layer(u32, bool),
    Other,
}

enum Arg<'a> {
    Uint(u32),
    Text(&'a str),
}

fn message(object: u32, opcode: u16, args: &[Arg<'_>]) -> Vec<u8> {
    let mut body = Vec::new();
    for arg in args {
        match arg {
            Arg::Uint(value) => body.extend_from_slice(&value.to_ne_bytes()),
            Arg::Text(text) => {
                let length = u32::try_from(text.len() + 1).expect("short text");
                body.extend_from_slice(&length.to_ne_bytes());
                body.extend_from_slice(text.as_bytes());
                body.push(0);
                while body.len() % 4 != 0 {
                    body.push(0);
                }
            }
        }
    }
    let size = u32::try_from(body.len() + 8).expect("short message");
    let mut out = object.to_ne_bytes().to_vec();
    out.extend_from_slice(&((size << 16) | u32::from(opcode)).to_ne_bytes());
    out.extend_from_slice(&body);
    out
}

fn word(bytes: &[u8], at: usize) -> u32 {
    u32::from_ne_bytes(bytes[at..at + 4].try_into().expect("four bytes"))
}

fn text_at(bytes: &[u8], at: usize) -> (String, usize) {
    let length = word(bytes, at) as usize;
    let text = String::from_utf8_lossy(&bytes[at + 4..at + 4 + length.saturating_sub(1)]);
    (text.into_owned(), at + 4 + length.div_ceil(4) * 4)
}

struct Server<'a> {
    objects: HashMap<u32, Kind>,
    frames: bool,
    pending_frames: HashMap<u32, Vec<u32>>,
    layers: HashMap<u32, u32>,
    log: &'a Mutex<Vec<String>>,
}

impl Server<'_> {
    fn note(&self, line: String) {
        if let Ok(mut log) = self.log.lock() {
            log.push(line);
        }
    }

    fn answer(&mut self, object: u32, opcode: u16, bytes: &[u8]) -> Vec<u8> {
        let kind = self.objects.get(&object).copied().unwrap_or(Kind::Other);
        match (kind, opcode) {
            (Kind::Display, 0) => done(word(bytes, 8)),
            (Kind::Display, 1) => {
                let registry = word(bytes, 8);
                self.objects.insert(registry, Kind::Registry);
                let mut out = Vec::new();
                for (index, (interface, version)) in ADVERTISED.iter().enumerate() {
                    let name = u32::try_from(index + 1).expect("few globals");
                    let args = [Arg::Uint(name), Arg::Text(interface), Arg::Uint(*version)];
                    out.extend(message(registry, 0, &args));
                }
                out
            }
            (Kind::Registry, 0) => {
                let (interface, at) = text_at(bytes, 12);
                let id = word(bytes, at + 4);
                self.note(format!("bind {interface}"));
                let kind = match interface.as_str() {
                    "wl_compositor" => Kind::Compositor,
                    "zwlr_layer_shell_v1" => Kind::LayerShell,
                    _ => Kind::Other,
                };
                self.objects.insert(id, kind);
                Vec::new()
            }
            (Kind::Compositor, 0) => self.create(word(bytes, 8), Kind::Surface),
            (Kind::Compositor, 1) => self.create(word(bytes, 8), Kind::Region(false)),
            (Kind::Region(_), 1) => self.create(object, Kind::Region(true)),
            (Kind::LayerShell, 0) => self.layer_surface(bytes),
            (Kind::Surface, 3) => {
                let callback = word(bytes, 8);
                self.pending_frames
                    .entry(object)
                    .or_default()
                    .push(callback);
                Vec::new()
            }
            (Kind::Surface, 5) => {
                let region = self.objects.get(&word(bytes, 8)).copied();
                let shape = match region {
                    Some(Kind::Region(false)) => "empty",
                    Some(Kind::Region(true)) => "shaped",
                    _ => "whole surface",
                };
                self.note(format!("input region {shape}"));
                Vec::new()
            }
            (Kind::Surface, 6) => self.commit(object),
            (Kind::Layer(..), _) => {
                self.layer_request(opcode, bytes);
                Vec::new()
            }
            _ => Vec::new(),
        }
    }

    fn create(&mut self, id: u32, kind: Kind) -> Vec<u8> {
        self.objects.insert(id, kind);
        Vec::new()
    }

    fn layer_surface(&mut self, bytes: &[u8]) -> Vec<u8> {
        let id = word(bytes, 8);
        let surface = word(bytes, 12);
        let output = word(bytes, 16);
        let layer = word(bytes, 20);
        let (namespace, _) = text_at(bytes, 24);
        self.note(format!(
            "layer {layer} namespace {namespace} output {output}"
        ));
        self.objects.insert(id, Kind::Layer(surface, false));
        self.layers.insert(surface, id);
        Vec::new()
    }

    fn layer_request(&self, opcode: u16, bytes: &[u8]) {
        let line = match opcode {
            1 => format!("anchor {}", word(bytes, 8)),
            2 => format!("exclusive zone {}", word(bytes, 8) as i32),
            3 => {
                let side = |at| word(bytes, at) as i32;
                format!("margin {} {} {} {}", side(8), side(12), side(16), side(20))
            }
            4 => format!("keyboard {}", word(bytes, 8)),
            6 => "ack configure".to_owned(),
            7 => "destroy layer".to_owned(),
            _ => return,
        };
        self.note(line);
    }

    fn commit(&mut self, surface: u32) -> Vec<u8> {
        self.note("commit".to_owned());
        let mut out = Vec::new();
        if let Some(&layer) = self.layers.get(&surface)
            && self.objects.get(&layer) == Some(&Kind::Layer(surface, false))
        {
            self.objects.insert(layer, Kind::Layer(surface, true));
            out.extend(message(
                layer,
                0,
                &[Arg::Uint(1), Arg::Uint(0), Arg::Uint(0)],
            ));
        }
        if self.frames {
            for callback in self.pending_frames.remove(&surface).unwrap_or_default() {
                out.extend(done(callback));
            }
        }
        out
    }
}

fn done(callback: u32) -> Vec<u8> {
    let mut out = message(callback, 0, &[Arg::Uint(0)]);
    out.extend(message(1, 1, &[Arg::Uint(callback)]));
    out
}

fn serve(mut stream: UnixStream, frames: bool, log: &Mutex<Vec<String>>) {
    let mut server = Server {
        objects: HashMap::from([(1, Kind::Display)]),
        frames,
        pending_frames: HashMap::new(),
        layers: HashMap::new(),
        log,
    };
    let mut pending = Vec::new();
    let mut chunk = [0_u8; 4096];
    loop {
        let read = match stream.read(&mut chunk) {
            Ok(0) | Err(_) => return,
            Ok(read) => read,
        };
        pending.extend_from_slice(&chunk[..read]);
        while pending.len() >= 8 {
            let size = (word(&pending, 4) >> 16) as usize;
            if pending.len() < size {
                break;
            }
            let bytes: Vec<u8> = pending.drain(..size).collect();
            let opcode = (word(&bytes, 4) & 0xffff) as u16;
            let reply = server.answer(word(&bytes, 0), opcode, &bytes);
            if stream.write_all(&reply).is_err() {
                return;
            }
        }
    }
}
