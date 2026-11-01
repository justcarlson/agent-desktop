//! How the renderer answers one request. `enable` updates the cursor's
//! state; `present` animates, or only moves the pose when parked, hidden or
//! held, and acknowledges travel after its arrival frame and an effect or
//! caret as it starts; `hide_cursor` commits a clear frame and `show_cursor`
//! redraws; `disable` answers and ends the process; `status` reports. Hide
//! holders are not served yet and are refused.

use std::os::unix::net::UnixStream;

use agent_desktop_core::{AdapterError, CursorOverlayControl, CursorPhase, Deadline, ErrorCode};
use serde_json::{Value, json};

use super::cursor_animation::{draw_at, travel};
use super::holder_view::{any_live, live_holders};
use super::renderer_process::RendererProcess;
use super::renderer_request::RendererRequest;
use super::renderer_wire::{error_reply, ok_reply, read_request, write_reply};

const CLEAR_WAIT_MS: u64 = 50;

/// Whether the renderer keeps serving after a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Flow {
    Continue,
    Exit,
}

/// Reads one request from `stream`, answers it, and says whether to go on.
pub(crate) fn serve_one(process: &mut RendererProcess, stream: &UnixStream) -> Flow {
    let request = match read_request(stream) {
        Ok(request) => request,
        Err(error) => {
            write_reply(stream, &error_reply("cursor_renderer_bad_request", &error));
            return Flow::Continue;
        }
    };
    match request {
        RendererRequest::Status => {
            write_reply(stream, &ok_reply(json!({"status": status(process)})));
            Flow::Continue
        }
        RendererRequest::Hide { .. } | RendererRequest::Show { .. } => {
            let refusal = AdapterError::not_supported("cursor hide holders");
            write_reply(
                stream,
                &error_reply("cursor_holders_not_supported", &refusal),
            );
            Flow::Continue
        }
        RendererRequest::Control { control, park } => {
            if control.session_id() != process.record.session_id {
                let refusal = AdapterError::new(
                    ErrorCode::InvalidArgs,
                    "The control names another session than this renderer serves",
                );
                write_reply(
                    stream,
                    &error_reply("cursor_renderer_session_mismatch", &refusal),
                );
                return Flow::Continue;
            }
            serve_control(process, stream, &control, park)
        }
    }
}

fn serve_control(
    process: &mut RendererProcess,
    stream: &UnixStream,
    control: &CursorOverlayControl,
    park: bool,
) -> Flow {
    let drawn = match control {
        CursorOverlayControl::Disable { .. } => {
            write_reply(stream, &ok_reply(json!({})));
            return Flow::Exit;
        }
        CursorOverlayControl::Enable { .. } => {
            process.cursor.adopt(control);
            Ok(())
        }
        CursorOverlayControl::Hide { .. } => {
            process.cursor.hidden = true;
            clear(process)
        }
        CursorOverlayControl::Show { .. } => {
            process.cursor.hidden = false;
            redraw(process)
        }
        CursorOverlayControl::Present { .. } => return present(process, stream, control, park),
    };
    answer(stream, drawn)
}

fn present(
    process: &mut RendererProcess,
    stream: &UnixStream,
    control: &CursorOverlayControl,
    park: bool,
) -> Flow {
    process.cursor.adopt(control);
    let Some(instruction) = control.instruction() else {
        return answer(stream, Ok(()));
    };
    let suppressed = park || process.cursor.hidden || any_live(&process.paths.holders);
    let travels = matches!(instruction.phase(), CursorPhase::Travel | CursorPhase::Drag);
    let to = match instruction.phase() {
        CursorPhase::Drag => instruction
            .drag_from()
            .unwrap_or(instruction.destination())
            .clone(),
        _ => instruction.destination().clone(),
    };
    if suppressed {
        process.cursor.pose = Some(to);
        return answer(stream, clear(process));
    }
    if travels {
        let arrived = travel(
            &mut process.client,
            &mut process.cursor,
            to,
            instruction.is_click(),
        );
        return answer(stream, arrived);
    }
    write_reply(stream, &ok_reply(json!({})));
    match draw_at(&mut process.client, &mut process.cursor, to) {
        Ok(()) => Flow::Continue,
        Err(_) => Flow::Exit,
    }
}

fn redraw(process: &mut RendererProcess) -> Result<(), AdapterError> {
    match process.cursor.pose.clone() {
        Some(pose) if !any_live(&process.paths.holders) => {
            draw_at(&mut process.client, &mut process.cursor, pose)
        }
        _ => Ok(()),
    }
}

fn clear(process: &mut RendererProcess) -> Result<(), AdapterError> {
    process
        .client
        .clear(Deadline::detached_after(CLEAR_WAIT_MS)?)
}

fn answer(stream: &UnixStream, drawn: Result<(), AdapterError>) -> Flow {
    match drawn {
        Ok(()) => {
            write_reply(stream, &ok_reply(json!({})));
            Flow::Continue
        }
        Err(error) => {
            write_reply(stream, &error_reply("cursor_renderer_draw_failed", &error));
            Flow::Exit
        }
    }
}

fn status(process: &RendererProcess) -> Value {
    let owner = &process.record.owner;
    json!({
        "pid": owner.pid,
        "start_time": owner.start_time,
        "generation": owner.generation,
        "protocol": process.record.protocol,
        "holders": live_holders(&process.paths.holders).len(),
        "cursors": 1,
        "outputs": process.client.output_count(),
    })
}
