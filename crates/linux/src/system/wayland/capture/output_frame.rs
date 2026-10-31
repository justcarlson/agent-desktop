//! One frame of one output through `zwlr_screencopy_v1`. The output is
//! chosen by the name `wl_output` version 4 announces, which is the id
//! `list-displays` reports. Overlay cursors are composited into an output
//! frame, so whoever draws them hides them before this runs.

use std::os::unix::net::UnixStream;

use agent_desktop_core::Deadline;
use wayland_client::protocol::wl_output::WlOutput;
use wayland_client::protocol::wl_shm::WlShm;
use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_frame_v1::ZwlrScreencopyFrameV1;
use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1;

use super::buffer_layout::BufferLayout;
use super::capture_fault::CaptureFault;
use super::capture_globals::{self, OUTPUT, SCREENCOPY, SHM};
use super::capture_state::CaptureState;
use super::frame_failure::FrameFailure;
use super::frame_freshness::monotonic_now;
use super::frame_outcome::FrameOutcome;
use super::frame_settle::settle_frame;
use super::raw_frame::RawFrame;
use super::shm_buffer::ShmBuffer;
use super::shm_layout::choose_format;
use crate::system::wayland::link::Link;

const NO_OVERLAY_CURSOR: i32 = 0;

type CaptureLink = Link<CaptureState>;

/// Captures one fresh frame of the output named `name` over `stream`.
pub(crate) fn capture_output(
    stream: UnixStream,
    name: &str,
    deadline: Deadline,
) -> Result<RawFrame, CaptureFault> {
    let mut state = CaptureState::default();
    let mut link = Link::over(stream, &mut state, deadline).map_err(CaptureFault::Setup)?;
    let chosen = capture_globals::output(&state.exchange).map_err(CaptureFault::Unsupported)?;
    let shm: WlShm = link.bind(&chosen.shm, SHM.1, ());
    let screencopy: ZwlrScreencopyManagerV1 = link.bind(&chosen.screencopy, SCREENCOPY.1, ());
    let outputs: Vec<(u32, WlOutput)> = chosen
        .outputs
        .iter()
        .map(|output| (output.name, link.bind(output, OUTPUT.1, output.name)))
        .collect();
    let outcome = link
        .roundtrip(&mut state)
        .map_err(CaptureFault::Setup)
        .and_then(|()| {
            let output = named(&state, &outputs, name)?;
            one_frame(&mut link, &mut state, &shm, &screencopy, &output)
        });
    for (_, output) in &outputs {
        output.release();
    }
    screencopy.destroy();
    let _ = link.flush();
    outcome
}

fn named(
    state: &CaptureState,
    outputs: &[(u32, WlOutput)],
    name: &str,
) -> Result<WlOutput, CaptureFault> {
    let mut matching = state
        .outputs
        .iter()
        .filter(|(_, announced)| announced == name);
    match (matching.next(), matching.next()) {
        (Some((registry_name, _)), None) => outputs
            .iter()
            .find(|(bound, _)| bound == registry_name)
            .map(|(_, output)| output.clone())
            .ok_or_else(|| CaptureFault::OutputMissing(name.to_owned())),
        _ => Err(CaptureFault::OutputMissing(name.to_owned())),
    }
}

fn one_frame(
    link: &mut CaptureLink,
    state: &mut CaptureState,
    shm: &WlShm,
    screencopy: &ZwlrScreencopyManagerV1,
    output: &WlOutput,
) -> Result<RawFrame, CaptureFault> {
    let frame = screencopy.capture_output(NO_OVERLAY_CURSOR, output, &link.handle(), ());
    let announced = link.wait(state, |state| {
        state.screencopy.offers_done || state.frame.outcome != FrameOutcome::Pending
    });
    let outcome = announced
        .map_err(CaptureFault::Setup)
        .and_then(|()| copy_into(link, state, shm, &frame));
    frame.destroy();
    outcome
}

fn copy_into(
    link: &mut CaptureLink,
    state: &mut CaptureState,
    shm: &WlShm,
    frame: &ZwlrScreencopyFrameV1,
) -> Result<RawFrame, CaptureFault> {
    if let FrameOutcome::Failed(failure) = state.frame.outcome {
        return Err(CaptureFault::Failed(failure));
    }
    let layout = offered(&state.screencopy.offers).ok_or(CaptureFault::NoFormat)?;
    let buffer = ShmBuffer::create(shm, layout, &link.handle()).ok_or(CaptureFault::NoFormat)?;
    let requested = monotonic_now();
    frame.copy(buffer.buffer());
    let waited = link.wait(state, |state| state.frame.outcome != FrameOutcome::Pending);
    settle_frame(waited, state.frame, requested)?;
    Ok(RawFrame {
        data: buffer
            .read()
            .ok_or(CaptureFault::Failed(FrameFailure::Unknown))?,
        format: layout.format,
        width: layout.width,
        height: layout.height,
        stride: layout.stride,
        inverted: state.screencopy.inverted,
    })
}

/// The offered buffer layout in the most preferred readable format.
pub(crate) fn offered(offers: &[BufferLayout]) -> Option<BufferLayout> {
    let formats: Vec<u32> = offers.iter().map(|offer| offer.format).collect();
    let format = choose_format(&formats)?;
    offers.iter().find(|offer| offer.format == format).copied()
}

#[cfg(test)]
#[path = "output_frame_tests.rs"]
mod tests;
