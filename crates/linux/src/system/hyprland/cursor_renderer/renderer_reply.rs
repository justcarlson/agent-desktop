//! How the renderer answers one request. Every control but `disable`
//! addresses one agent's cursor; in a multi-agent session a control that
//! names no agent is refused, and a named one retires the session's
//! unnamed cursor. `enable` updates that cursor's style and redraws it where
//! it is, without moving it; `present` starts a travel, acknowledged after
//! its arrival frame, or only moves the pose when parked, hidden, held or
//! bound for a point on no output, where a parked cursor stays undrawn by
//! any redraw until a present aims at a target that shows; it acknowledges
//! an effect or caret as
//! it starts; a newer `present` or
//! `hide_cursor` for the same agent supersedes its travel in flight;
//! `hide_cursor` commits a clear frame and `show_cursor` redraws; `disable`
//! answers and ends the process; `status` reports every cursor. A holder's
//! `hide` and `show` go to the holder watch.

use std::os::unix::net::UnixStream;

use agent_desktop_core::{
    AdapterError, CURSOR_MOTION_BUDGET_MS, CursorOverlayControl, CursorPhase, Deadline, ErrorCode,
};
use serde_json::{Value, json};

use super::agent_cursors::{AgentCursors, SOLE_CURSOR};
use super::cursor_animation::draw_at;
use super::cursor_travel::CursorTravel;
use super::holder_reply::holder_reply;
use super::holder_view::{any_live, live_holders};
use super::holder_watch::HolderWatch;
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
/// A travel keeps `stream` until its arrival frame.
pub(crate) fn serve_one(
    process: &mut RendererProcess,
    watch: &mut HolderWatch,
    stream: UnixStream,
) -> Flow {
    let request = match read_request(&stream) {
        Ok(request) => request,
        Err(error) => {
            write_reply(&stream, &error_reply("cursor_renderer_bad_request", &error));
            return Flow::Continue;
        }
    };
    match request {
        RendererRequest::Status => {
            write_reply(&stream, &ok_reply(json!({"status": status(process)})));
            Flow::Continue
        }
        RendererRequest::Hide { .. } | RendererRequest::Show { .. } => {
            let (reply, flow) = holder_reply(process, watch, &request);
            write_reply(&stream, &reply);
            flow
        }
        RendererRequest::Control { control, park } => {
            if control.session_id() != process.record.session_id {
                let refusal = AdapterError::new(
                    ErrorCode::InvalidArgs,
                    "The control names another session than this renderer serves",
                );
                write_reply(
                    &stream,
                    &error_reply("cursor_renderer_session_mismatch", &refusal),
                );
                return Flow::Continue;
            }
            serve_control(process, stream, &control, park)
        }
    }
}

/// Acknowledges a travel that drew its arrival frame.
pub(crate) fn arrived(stream: &UnixStream) {
    write_reply(stream, &ok_reply(json!({})));
}

/// Refuses a travel whose arrival frame no output showed in time, so the
/// caller is never told the cursor arrived when that is unknown.
pub(crate) fn arrival_unconfirmed(stream: &UnixStream) {
    let refusal = AdapterError::new(
        ErrorCode::Timeout,
        "No output showed the cursor's arrival frame before the arrival timeout",
    );
    write_reply(stream, &error_reply("cursor_arrival_unconfirmed", &refusal));
}

/// Answers every travel in flight with a refusal of `kind`.
pub(crate) fn answer_travels(cursors: &mut AgentCursors, kind: &str) {
    let refusal = AdapterError::new(
        ErrorCode::ActionFailed,
        "The cursor did not arrive before the renderer stopped",
    );
    for (_, cursor) in cursors.iter_mut() {
        if let Some(travel) = cursor.travel.take() {
            write_reply(&travel.into_reply(), &error_reply(kind, &refusal));
        }
    }
}

fn serve_control(
    process: &mut RendererProcess,
    stream: UnixStream,
    control: &CursorOverlayControl,
    park: bool,
) -> Flow {
    if control.is_disable() {
        write_reply(&stream, &ok_reply(json!({})));
        return Flow::Exit;
    }
    let key = match AgentCursors::key_for(control, || process.multi_agent()) {
        Ok(key) => key,
        Err(refusal) => {
            write_reply(&stream, &error_reply("cursor_agent_id_required", &refusal));
            return Flow::Continue;
        }
    };
    if key != SOLE_CURSOR {
        retire_sole_cursor(process);
    }
    let cursor = process.cursors.cursor(&key, control);
    let drawn = match control {
        CursorOverlayControl::Present { .. } => {
            return present(process, stream, &key, control, park);
        }
        CursorOverlayControl::Hide { .. } => {
            cursor.concealed.hidden = true;
            supersede(process, &key);
            clear(process, &key)
        }
        CursorOverlayControl::Show { .. } => {
            cursor.concealed.hidden = false;
            process.redraw(&key)
        }
        _ => process.redraw(&key),
    };
    answer(&stream, drawn)
}

fn present(
    process: &mut RendererProcess,
    stream: UnixStream,
    key: &str,
    control: &CursorOverlayControl,
    park: bool,
) -> Flow {
    let Some(instruction) = control.instruction() else {
        return answer(&stream, Ok(()));
    };
    supersede(process, key);
    let travels = matches!(instruction.phase(), CursorPhase::Travel | CursorPhase::Drag);
    let to = match instruction.phase() {
        CursorPhase::Drag => instruction
            .drag_from()
            .unwrap_or(instruction.destination())
            .clone(),
        _ => instruction.destination().clone(),
    };
    let held = any_live(&process.paths.holders);
    let off_layout = !process.client.on_output(&to);
    let Some(cursor) = process.cursors.get_mut(key) else {
        return answer(&stream, Ok(()));
    };
    cursor.concealed.parked = park || off_layout;
    if cursor.concealed.any() || held {
        cursor.pose = Some(to);
        return answer(&stream, clear(process, key));
    }
    if travels {
        let budget = match Deadline::detached_after(CURSOR_MOTION_BUDGET_MS) {
            Ok(budget) => budget,
            Err(error) => return answer(&stream, Err(error)),
        };
        let travel = CursorTravel::begin(cursor, to, instruction.is_click(), budget, stream);
        cursor.travel = Some(travel);
        return Flow::Continue;
    }
    write_reply(&stream, &ok_reply(json!({})));
    match draw_at(&mut process.client, key, cursor, to) {
        Ok(()) => Flow::Continue,
        Err(_) => {
            answer_travels(&mut process.cursors, "cursor_renderer_draw_failed");
            Flow::Exit
        }
    }
}

fn supersede(process: &mut RendererProcess, key: &str) {
    let Some(travel) = process
        .cursors
        .get_mut(key)
        .and_then(|cursor| cursor.travel.take())
    else {
        return;
    };
    let refusal = AdapterError::new(
        ErrorCode::ActionFailed,
        "A newer cursor control for this agent replaced the travel",
    );
    write_reply(
        &travel.into_reply(),
        &error_reply("cursor_travel_superseded", &refusal),
    );
}

fn retire_sole_cursor(process: &mut RendererProcess) {
    supersede(process, SOLE_CURSOR);
    if process.cursors.remove(SOLE_CURSOR).is_some() {
        process.client.forget(SOLE_CURSOR);
    }
}

fn clear(process: &mut RendererProcess, key: &str) -> Result<(), AdapterError> {
    process
        .client
        .clear(key, Deadline::detached_after(CLEAR_WAIT_MS)?)
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
        "cursors": process.cursors.len(),
        "agents": process.cursors.agents(),
        "outputs": process.client.output_count(),
    })
}

#[cfg(test)]
#[path = "renderer_reply_tests.rs"]
mod tests;
