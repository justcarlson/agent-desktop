//! A scripted compositor for unit tests. It speaks just enough of the
//! Wayland wire protocol to answer a registry read, `wl_output` binds,
//! `zxdg_output_manager_v1.get_xdg_output` and the toplevel globals in
//! `fake_toplevels`, or it accepts the connection and stays silent, or hangs
//! up. Every request a toplevel handle receives is logged.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use super::fake_toplevels::{self, FakeToplevel, HandleIds};

const OUTPUT: &str = "wl_output";
const MANAGER: &str = "zxdg_output_manager_v1";

/// One output the fake compositor reports.
#[derive(Debug, Clone)]
pub(crate) struct FakeOutput {
    pub(crate) name: &'static str,
    pub(crate) position: (i32, i32),
    pub(crate) logical: (i32, i32),
    pub(crate) mode: (i32, i32),
    pub(crate) transform: i32,
    pub(crate) scale: i32,
}

/// What the fake compositor does with the one connection it accepts.
#[derive(Debug, Clone)]
pub(crate) enum Script {
    Answer {
        globals: Vec<(&'static str, u32)>,
        outputs: Vec<FakeOutput>,
    },
    Toplevels {
        globals: Vec<(&'static str, u32)>,
        toplevels: Vec<FakeToplevel>,
    },
    Silent,
    HangUp,
}

/// A listening fake compositor; its socket directory goes when it drops.
pub(crate) struct FakeCompositor {
    pub(crate) socket: PathBuf,
    pub(crate) log: Arc<Mutex<Vec<String>>>,
    dir: PathBuf,
}

impl FakeCompositor {
    pub(crate) fn start(script: Script) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "agent-desktop-fake-wayland-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).expect("socket directory");
        let socket = dir.join("wayland-fake");
        let listener = UnixListener::bind(&socket).expect("bind fake compositor");
        let log = Arc::new(Mutex::new(Vec::new()));
        let served = Arc::clone(&log);
        std::thread::spawn(move || {
            let Ok((stream, _)) = listener.accept() else {
                return;
            };
            match script {
                Script::Answer { globals, outputs } => {
                    serve(stream, &globals, &outputs, Vec::new(), served)
                }
                Script::Toplevels { globals, toplevels } => {
                    serve(stream, &globals, &[], toplevels, served)
                }
                Script::Silent => {
                    let _ = (&stream).read_to_end(&mut Vec::new());
                }
                Script::HangUp => drop(stream),
            }
        });
        Self { socket, log, dir }
    }
}

impl Drop for FakeCompositor {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

pub(super) enum Arg<'a> {
    Uint(u32),
    Int(i32),
    Text(&'a str),
    Array(&'a [u8]),
}

#[derive(Clone, Copy)]
enum Kind {
    Display,
    Registry,
    Output(usize, u32),
    Manager(u32),
    List,
    Mapper,
    WlrManager,
    WlrHandle(usize),
    Other,
}

pub(super) fn message(object: u32, opcode: u16, args: &[Arg<'_>]) -> Vec<u8> {
    let mut body = Vec::new();
    for arg in args {
        match arg {
            Arg::Uint(value) => body.extend_from_slice(&value.to_ne_bytes()),
            Arg::Int(value) => body.extend_from_slice(&value.to_ne_bytes()),
            Arg::Text(text) => {
                let length = u32::try_from(text.len() + 1).expect("short text");
                body.extend_from_slice(&length.to_ne_bytes());
                body.extend_from_slice(text.as_bytes());
                body.push(0);
                pad(&mut body);
            }
            Arg::Array(bytes) => {
                let length = u32::try_from(bytes.len()).expect("short array");
                body.extend_from_slice(&length.to_ne_bytes());
                body.extend_from_slice(bytes);
                pad(&mut body);
            }
        }
    }
    let size = u32::try_from(body.len() + 8).expect("short message");
    let mut out = object.to_ne_bytes().to_vec();
    out.extend_from_slice(&((size << 16) | u32::from(opcode)).to_ne_bytes());
    out.extend_from_slice(&body);
    out
}

fn pad(body: &mut Vec<u8>) {
    while body.len() % 4 != 0 {
        body.push(0);
    }
}

fn word(bytes: &[u8], at: usize) -> u32 {
    u32::from_ne_bytes(bytes[at..at + 4].try_into().expect("four bytes"))
}

fn text_at(bytes: &[u8], at: usize) -> (String, usize) {
    let length = word(bytes, at) as usize;
    let text = String::from_utf8_lossy(&bytes[at + 4..at + 4 + length.saturating_sub(1)]);
    (text.into_owned(), at + 4 + length.div_ceil(4) * 4)
}

fn output_events(id: u32, version: u32, output: &FakeOutput) -> Vec<u8> {
    let mut out = message(
        id,
        0,
        &[
            Arg::Int(output.position.0),
            Arg::Int(output.position.1),
            Arg::Int(300),
            Arg::Int(200),
            Arg::Int(0),
            Arg::Text("fake"),
            Arg::Text("fake"),
            Arg::Int(output.transform),
        ],
    );
    let (width, height) = output.mode;
    out.extend(message(
        id,
        1,
        &[
            Arg::Uint(3),
            Arg::Int(width),
            Arg::Int(height),
            Arg::Int(60_000),
        ],
    ));
    if version >= 2 {
        out.extend(message(id, 3, &[Arg::Int(output.scale)]));
    }
    if version >= 4 {
        out.extend(message(id, 4, &[Arg::Text(output.name)]));
    }
    if version >= 2 {
        out.extend(message(id, 2, &[]));
    }
    out
}

fn xdg_events(id: u32, manager: u32, output: (u32, u32), fake: &FakeOutput) -> Vec<u8> {
    let (x, y) = fake.position;
    let (width, height) = fake.logical;
    let mut out = message(id, 0, &[Arg::Int(x), Arg::Int(y)]);
    out.extend(message(id, 1, &[Arg::Int(width), Arg::Int(height)]));
    if manager >= 2 {
        out.extend(message(id, 3, &[Arg::Text(fake.name)]));
    }
    if manager < 3 {
        out.extend(message(id, 2, &[]));
    } else if output.1 >= 2 {
        out.extend(message(output.0, 2, &[]));
    }
    out
}

struct Server<'a> {
    globals: &'a [(&'static str, u32)],
    outputs: &'a [FakeOutput],
    toplevels: Vec<FakeToplevel>,
    objects: HashMap<u32, Kind>,
    ids: HandleIds,
    log: Arc<Mutex<Vec<String>>>,
}

impl Server<'_> {
    fn answer(&mut self, object: u32, opcode: u16, bytes: &[u8]) -> Vec<u8> {
        let kind = self.objects.get(&object).copied().unwrap_or(Kind::Other);
        match (kind, opcode) {
            (Kind::Display, 0) => {
                let callback = word(bytes, 8);
                let mut out = message(callback, 0, &[Arg::Uint(0)]);
                out.extend(message(1, 1, &[Arg::Uint(callback)]));
                out
            }
            (Kind::Display, 1) => {
                self.objects.insert(word(bytes, 8), Kind::Registry);
                let registry = word(bytes, 8);
                let mut out = Vec::new();
                for (index, (interface, version)) in self.globals.iter().enumerate() {
                    let name = u32::try_from(index + 1).expect("few globals");
                    let args = [Arg::Uint(name), Arg::Text(interface), Arg::Uint(*version)];
                    out.extend(message(registry, 0, &args));
                }
                out
            }
            (Kind::Registry, 0) => self.bind(bytes),
            (Kind::Manager(manager), 1) => {
                let id = word(bytes, 8);
                let output = word(bytes, 12);
                self.objects.insert(id, Kind::Other);
                match self.objects.get(&output) {
                    Some(Kind::Output(index, version)) => {
                        xdg_events(id, manager, (output, *version), &self.outputs[*index])
                    }
                    _ => Vec::new(),
                }
            }
            (Kind::Mapper, 0 | 1) => {
                let first = if opcode == 0 {
                    self.ids.ext
                } else {
                    self.ids.wlr
                };
                let target = (first, word(bytes, 12));
                fake_toplevels::mapping_reply(word(bytes, 8), target, &self.toplevels)
            }
            (Kind::WlrHandle(index), request) => {
                if let Ok(mut log) = self.log.lock() {
                    log.push(fake_toplevels::logged(request, &self.toplevels[index]));
                }
                let at = (self.ids.wlr, index);
                fake_toplevels::handle_request(at, request, &mut self.toplevels)
            }
            _ => Vec::new(),
        }
    }

    fn bind(&mut self, bytes: &[u8]) -> Vec<u8> {
        let name = word(bytes, 8) as usize;
        let (interface, at) = text_at(bytes, 12);
        let version = word(bytes, at);
        let id = word(bytes, at + 4);
        let outputs_before = self.globals[..name - 1]
            .iter()
            .filter(|(each, _)| *each == OUTPUT)
            .count();
        match interface.as_str() {
            OUTPUT => {
                self.objects
                    .insert(id, Kind::Output(outputs_before, version));
                output_events(id, version, &self.outputs[outputs_before])
            }
            MANAGER => {
                self.objects.insert(id, Kind::Manager(version));
                Vec::new()
            }
            fake_toplevels::LIST => {
                self.objects.insert(id, Kind::List);
                self.ids.ext = self.ids.take(self.toplevels.len());
                fake_toplevels::list_events(id, self.ids.ext, &self.toplevels)
            }
            fake_toplevels::WLR_MANAGER => {
                self.objects.insert(id, Kind::WlrManager);
                self.ids.wlr = self.ids.take(self.toplevels.len());
                for index in 0..self.toplevels.len() {
                    let handle = self.ids.wlr + u32::try_from(index).expect("few toplevels");
                    self.objects.insert(handle, Kind::WlrHandle(index));
                }
                fake_toplevels::manager_events((id, version), self.ids.wlr, &self.toplevels)
            }
            fake_toplevels::MAPPER => {
                self.objects.insert(id, Kind::Mapper);
                Vec::new()
            }
            _ => {
                self.objects.insert(id, Kind::Other);
                Vec::new()
            }
        }
    }
}

fn serve(
    mut stream: UnixStream,
    globals: &[(&'static str, u32)],
    outputs: &[FakeOutput],
    toplevels: Vec<FakeToplevel>,
    log: Arc<Mutex<Vec<String>>>,
) {
    let mut server = Server {
        globals,
        outputs,
        toplevels,
        objects: HashMap::from([(1, Kind::Display)]),
        ids: HandleIds::default(),
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
