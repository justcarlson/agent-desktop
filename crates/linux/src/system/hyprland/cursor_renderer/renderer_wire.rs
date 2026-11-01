//! The transport: a Unix stream socket, one JSON request of at most 4 KiB per
//! connection with the write side shut, and one JSON line back:
//! `{"ok":true,...}` or `{"ok":false,"error":{"kind":...,"message":...}}`.

use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::{Duration, Instant};

use agent_desktop_core::{AdapterError, ErrorCode};
use serde_json::{Value, json};

use super::renderer_request::RendererRequest;

/// The largest request a renderer reads.
pub(crate) const MAX_REQUEST_BYTES: u64 = 4 * 1024;

/// Sends `request` to the renderer at `socket` and returns its reply, or
/// `None` when nothing listens there.
pub(crate) fn exchange(
    socket: &Path,
    request: &RendererRequest,
    deadline: Instant,
) -> Result<Option<Value>, AdapterError> {
    let payload = request.to_json()?.to_string();
    if payload.len() as u64 > MAX_REQUEST_BYTES {
        return Err(AdapterError::new(
            ErrorCode::InvalidArgs,
            "Cursor renderer request exceeds the transport limit",
        ));
    }
    let mut stream = match UnixStream::connect(socket) {
        Ok(stream) => stream,
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::NotFound | std::io::ErrorKind::ConnectionRefused
            ) =>
        {
            return Ok(None);
        }
        Err(error) => return Err(io_error("Could not reach the cursor renderer", error)),
    };
    let remaining = time_left(deadline)?;
    stream
        .set_write_timeout(Some(remaining))
        .and_then(|()| stream.write_all(payload.as_bytes()))
        .and_then(|()| stream.shutdown(std::net::Shutdown::Write))
        .map_err(|error| io_error("Could not send the cursor renderer request", error))?;
    stream
        .set_read_timeout(Some(time_left(deadline)?))
        .map_err(|error| io_error("Could not bound the cursor renderer reply", error))?;
    let mut line = String::new();
    BufReader::new(stream.take(MAX_REQUEST_BYTES))
        .read_line(&mut line)
        .map_err(|error| reply_error(request, error))?;
    let reply: Value = serde_json::from_str(line.trim_end()).map_err(|error| {
        AdapterError::internal("The cursor renderer closed without a reply")
            .with_platform_detail(error.to_string())
    })?;
    if reply.get("ok").and_then(Value::as_bool) == Some(true) {
        return Ok(Some(reply));
    }
    Err(refused(&reply))
}

/// Reads one request from an accepted connection.
pub(crate) fn read_request(stream: &UnixStream) -> Result<RendererRequest, AdapterError> {
    stream
        .set_read_timeout(Some(Duration::from_secs(1)))
        .map_err(|error| io_error("Could not bound the renderer request read", error))?;
    let mut payload = Vec::new();
    stream
        .take(MAX_REQUEST_BYTES + 1)
        .read_to_end(&mut payload)
        .map_err(|error| io_error("Could not read the renderer request", error))?;
    if payload.len() as u64 > MAX_REQUEST_BYTES {
        return Err(AdapterError::new(
            ErrorCode::InvalidArgs,
            "Cursor renderer request exceeds the transport limit",
        ));
    }
    RendererRequest::parse(&payload)
}

/// Writes one reply line; the peer may already be gone.
pub(crate) fn write_reply(mut stream: &UnixStream, reply: &Value) {
    let _ = stream.set_write_timeout(Some(Duration::from_secs(1)));
    let _ = stream.write_all(format!("{reply}\n").as_bytes());
}

/// The success reply, with `extra` fields merged in.
pub(crate) fn ok_reply(extra: Value) -> Value {
    let mut reply = json!({"ok": true});
    if let (Some(map), Value::Object(extra)) = (reply.as_object_mut(), extra) {
        map.extend(extra);
    }
    reply
}

/// The refusal reply for `error`.
pub(crate) fn error_reply(kind: &str, error: &AdapterError) -> Value {
    json!({"ok": false, "error": {"kind": kind, "message": error.message}})
}

fn refused(reply: &Value) -> AdapterError {
    let kind = reply["error"]["kind"]
        .as_str()
        .unwrap_or("cursor_renderer_failed");
    let message = reply["error"]["message"].as_str().unwrap_or("no message");
    AdapterError::new(
        ErrorCode::ActionFailed,
        "The cursor renderer refused the request",
    )
    .with_platform_detail(message.to_owned())
    .with_details(json!({"kind": kind}))
}

fn time_left(deadline: Instant) -> Result<Duration, AdapterError> {
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        return Err(AdapterError::timeout(
            "The cursor renderer request deadline elapsed",
        ));
    }
    Ok(remaining)
}

fn reply_error(request: &RendererRequest, error: std::io::Error) -> AdapterError {
    if matches!(
        error.kind(),
        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
    ) {
        return AdapterError::timeout(format!(
            "The cursor renderer did not acknowledge {} in time",
            request.message()
        ));
    }
    io_error("Could not read the cursor renderer reply", error)
}

fn io_error(message: &str, error: std::io::Error) -> AdapterError {
    AdapterError::internal(message.to_owned()).with_platform_detail(error.to_string())
}

#[cfg(test)]
#[path = "renderer_wire_tests.rs"]
mod tests;
