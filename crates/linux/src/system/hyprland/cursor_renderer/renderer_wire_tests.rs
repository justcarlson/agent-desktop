use std::os::unix::net::UnixListener;
use std::time::{Duration, Instant};

use serde_json::json;

use super::{error_reply, exchange, ok_reply, read_request, write_reply};
use crate::system::hyprland::cursor_renderer::registry_fixture::RegistryFixture;
use crate::system::hyprland::cursor_renderer::renderer_request::RendererRequest;

fn soon() -> Instant {
    Instant::now() + Duration::from_secs(2)
}

#[test]
fn nothing_listening_is_none_not_an_error() {
    let fixture = RegistryFixture::new();
    let reply = exchange(&fixture.paths.socket, &RendererRequest::Status, soon()).expect("ok");
    assert!(reply.is_none());
}

#[test]
fn one_request_gets_one_reply_line() {
    let fixture = RegistryFixture::new();
    let listener = UnixListener::bind(&fixture.paths.socket).expect("bind");
    let server = std::thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept");
        let request = read_request(&stream).expect("request");
        write_reply(&stream, &ok_reply(json!({"status": {"pid": 7}})));
        request
    });
    let reply = exchange(&fixture.paths.socket, &RendererRequest::Status, soon())
        .expect("exchange")
        .expect("reply");
    assert_eq!(reply["status"]["pid"], 7);
    assert_eq!(server.join().expect("server"), RendererRequest::Status);
}

#[test]
fn a_refusal_carries_its_kind() {
    let fixture = RegistryFixture::new();
    let listener = UnixListener::bind(&fixture.paths.socket).expect("bind");
    std::thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept");
        let _ = read_request(&stream);
        let error = agent_desktop_core::AdapterError::internal("no");
        write_reply(&stream, &error_reply("cursor_renderer_busy", &error));
    });
    let error =
        exchange(&fixture.paths.socket, &RendererRequest::Status, soon()).expect_err("refused");
    assert_eq!(
        error.details.expect("details")["kind"],
        "cursor_renderer_busy"
    );
}

#[test]
fn a_silent_renderer_times_out_within_the_deadline() {
    let fixture = RegistryFixture::new();
    let listener = UnixListener::bind(&fixture.paths.socket).expect("bind");
    let held = std::thread::spawn(move || listener.accept().map(|(stream, _)| stream));
    let started = Instant::now();
    let error = exchange(
        &fixture.paths.socket,
        &RendererRequest::Status,
        Instant::now() + Duration::from_millis(200),
    )
    .expect_err("silent");
    assert_eq!(error.code, agent_desktop_core::ErrorCode::Timeout);
    assert!(started.elapsed() < Duration::from_secs(2));
    drop(held);
}
