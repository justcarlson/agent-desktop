use std::io::{Read, Write};
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::thread;
use std::time::Instant;

use agent_desktop_core::Deadline;

use super::*;

fn listener(name: &str) -> (PathBuf, UnixListener) {
    let dir = std::env::temp_dir().join(format!("ipc-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("dir");
    let path = dir.join(".socket.sock");
    let _ = std::fs::remove_file(&path);
    let listener = UnixListener::bind(&path).expect("bind");
    (path, listener)
}

fn deadline(ms: u64) -> Deadline {
    Deadline::after(ms).expect("deadline")
}

fn answer_with(listener: UnixListener, reply: Vec<u8>) -> thread::JoinHandle<String> {
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut command = [0_u8; 64];
        let read = stream.read(&mut command).expect("read");
        stream.write_all(&reply).expect("write");
        String::from_utf8_lossy(&command[..read]).into_owned()
    })
}

#[test]
fn a_reply_is_read_to_the_end_and_names_its_server() {
    let (path, listener) = listener("reply");
    let server = answer_with(listener, br#"[{"name":"HEADLESS-1"}]"#.to_vec());
    let reply = request(&path, "j/monitors", deadline(1_000)).expect("reply");
    assert_eq!(server.join().expect("server"), "j/monitors");
    assert_eq!(reply.pid, std::process::id());
    assert_eq!(reply.body, br#"[{"name":"HEADLESS-1"}]"#);
}

#[test]
fn a_server_that_never_replies_is_silent_within_the_deadline() {
    let (path, listener) = listener("silent");
    let held = thread::spawn(move || {
        let accepted = listener.accept().expect("accept");
        thread::sleep(std::time::Duration::from_millis(900));
        drop(accepted);
    });
    let started = Instant::now();
    assert_eq!(
        request(&path, "j/version", deadline(200)),
        Err(IpcFault::Silent)
    );
    assert!(
        started.elapsed().as_millis() < 700,
        "{:?}",
        started.elapsed()
    );
    held.join().expect("held");
}

#[test]
fn a_server_that_never_accepts_is_silent_within_the_deadline() {
    let (path, _listener) = listener("unaccepted");
    let started = Instant::now();
    assert_eq!(
        request(&path, "j/version", deadline(200)),
        Err(IpcFault::Silent)
    );
    assert!(
        started.elapsed().as_millis() < 700,
        "{:?}",
        started.elapsed()
    );
}

#[test]
fn a_socket_without_a_server_is_gone_and_an_empty_reply_is_gone() {
    let (path, listener) = listener("gone");
    let server = answer_with(listener, Vec::new());
    assert_eq!(
        request(&path, "j/version", deadline(1_000)),
        Err(IpcFault::Gone)
    );
    server.join().expect("server");
    assert_eq!(
        request(
            &path.with_file_name("none.sock"),
            "j/version",
            deadline(1_000)
        ),
        Err(IpcFault::Gone)
    );
}

#[test]
fn a_reply_past_the_cap_is_refused() {
    let (path, listener) = listener("oversized");
    let server = answer_with(listener, vec![b'x'; REPLY_CAP + 1]);
    assert_eq!(
        request(&path, "j/clients", deadline(2_000)),
        Err(IpcFault::Oversized)
    );
    let _ = server.join();
}
