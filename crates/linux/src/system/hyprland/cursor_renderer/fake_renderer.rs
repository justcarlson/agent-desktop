//! A stand-in renderer for caller-side hide tests: it records itself for this
//! test process, listens on its session socket and answers each request as
//! told, logging the message names it read.

use std::io::Read;
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use serde_json::{Value, json};

use super::registry_fixture::record_for;
use super::renderer_paths::RendererPaths;
use super::renderer_wire::write_reply;

/// How a fake renderer answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FakeAnswer {
    Acknowledge,
    Withhold,
    Refuse,
}

pub(crate) struct FakeRenderer {
    pub(crate) paths: RendererPaths,
    log: Arc<Mutex<Vec<Value>>>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl FakeRenderer {
    /// Starts a fake renderer for session `key` in `dir`.
    pub(crate) fn start(dir: &std::path::Path, key: &str, answer: FakeAnswer) -> Self {
        let paths = RendererPaths::in_dir(dir, key).expect("paths");
        let listener = UnixListener::bind(&paths.socket).expect("bind");
        record_for(std::process::id(), None, key)
            .write_atomic(&paths.record)
            .expect("record");
        let log = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let thread = {
            let (log, stop) = (Arc::clone(&log), Arc::clone(&stop));
            std::thread::spawn(move || serve(&listener, answer, &log, &stop))
        };
        Self {
            paths,
            log,
            stop,
            thread: Some(thread),
        }
    }

    /// The `message` of every request read so far, in order.
    pub(crate) fn messages(&self) -> Vec<String> {
        self.requests()
            .iter()
            .filter_map(|request| request["message"].as_str().map(str::to_owned))
            .collect()
    }

    pub(crate) fn requests(&self) -> Vec<Value> {
        self.log.lock().expect("log").clone()
    }
}

impl Drop for FakeRenderer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = UnixStream::connect(&self.paths.socket);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn serve(listener: &UnixListener, answer: FakeAnswer, log: &Mutex<Vec<Value>>, stop: &AtomicBool) {
    let mut held = Vec::new();
    while let Ok((mut stream, _)) = listener.accept() {
        if stop.load(Ordering::SeqCst) {
            return;
        }
        let mut payload = Vec::new();
        let _ = (&mut stream).take(4096).read_to_end(&mut payload);
        if let Ok(request) = serde_json::from_slice::<Value>(&payload) {
            log.lock().expect("log").push(request);
        }
        match answer {
            FakeAnswer::Acknowledge => write_reply(&stream, &json!({"ok": true})),
            FakeAnswer::Refuse => write_reply(
                &stream,
                &json!({"ok": false, "error": {"kind": "cursor_renderer_draw_failed", "message": "no"}}),
            ),
            FakeAnswer::Withhold => held.push(stream),
        }
    }
}
