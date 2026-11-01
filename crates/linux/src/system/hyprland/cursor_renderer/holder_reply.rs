//! How the renderer answers `hide` and `show` from a holder, and follows the
//! holders on each idle tick, over its own surfaces.

use agent_desktop_core::{AdapterError, Deadline};
use serde_json::{Value, json};

use super::holder_watch::{HolderCanvas, HolderWatch};
use super::renderer_process::RendererProcess;
use super::renderer_reply::Flow;
use super::renderer_request::RendererRequest;
use super::renderer_wire::{error_reply, ok_reply};

/// How long the renderer waits for the frame of a clear commit and for the
/// compositor to handle it.
#[cfg_attr(not(test), allow(dead_code))]
const HOLDER_WAIT_MS: u64 = 200;

/// The renderer's surfaces as a holder canvas.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) struct ProcessCanvas<'a> {
    pub(crate) process: &'a mut RendererProcess,
}

impl HolderCanvas for ProcessCanvas<'_> {
    fn clear_all(&mut self) -> Result<(), AdapterError> {
        let deadline = Deadline::detached_after(HOLDER_WAIT_MS)?;
        cursor_keys(self.process)
            .iter()
            .try_for_each(|key| self.process.client.clear(key, deadline))
    }

    fn sync(&mut self) -> Result<(), AdapterError> {
        self.process.client.pump()
    }

    fn redraw(&mut self) -> Result<(), AdapterError> {
        cursor_keys(self.process)
            .iter()
            .try_for_each(|key| self.process.redraw(key))
    }
}

fn cursor_keys(process: &mut RendererProcess) -> Vec<String> {
    process
        .cursors
        .iter_mut()
        .map(|(key, _)| key.clone())
        .collect()
}

/// The reply to a holder's `hide` or `show`, and whether the renderer keeps
/// serving: a cursor it cannot clear or draw ends the process, which takes
/// its surfaces with it.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn holder_reply(
    process: &mut RendererProcess,
    watch: &mut HolderWatch,
    request: &RendererRequest,
) -> (Value, Flow) {
    let holders = process.paths.holders.clone();
    let mut canvas = ProcessCanvas { process };
    let answered = match request {
        RendererRequest::Hide { holder, .. } => watch.hide(&holders, holder, &mut canvas),
        RendererRequest::Show { .. } => watch.show(&holders, &mut canvas),
        RendererRequest::Status | RendererRequest::Control { .. } => Ok(()),
    };
    match answered {
        Ok(()) => (ok_reply(json!({})), Flow::Continue),
        Err((kind, error)) => {
            let flow = if kind == "cursor_renderer_draw_failed" {
                Flow::Exit
            } else {
                Flow::Continue
            };
            (error_reply(kind, &error), flow)
        }
    }
}

/// Follows the holders on an idle tick. An error means the cursor could not
/// be cleared or drawn, and the renderer ends.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn refresh_holders(
    process: &mut RendererProcess,
    watch: &mut HolderWatch,
) -> Result<(), AdapterError> {
    let holders = process.paths.holders.clone();
    watch.refresh(&holders, &mut ProcessCanvas { process })
}

#[cfg(test)]
#[path = "holder_reply_tests.rs"]
mod tests;
