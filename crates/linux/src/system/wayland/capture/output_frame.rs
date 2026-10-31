//! One frame of one output through `zwlr_screencopy_v1`. The output is
//! chosen by the name `wl_output` version 4 announces, which is the name
//! Hyprland's monitor list reports. Overlay cursors are composited into an
//! output frame, so whoever draws them hides them before this runs.

use agent_desktop_core::Deadline;
use wayland_client::Connection;
use wayland_client::protocol::wl_output::WlOutput;
use wayland_client::protocol::wl_shm::WlShm;
use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1;

use super::buffer_layout::BufferLayout;
use super::capture_fault::CaptureFault;
use super::capture_globals;
use super::capture_queue::CaptureQueue;
use super::frame_failure::FrameFailure;
use super::frame_freshness::monotonic_now;
use super::frame_outcome::FrameOutcome;
use super::frame_settle::settle_frame;
use super::raw_frame::RawFrame;
use super::shm_buffer::ShmBuffer;
use super::shm_layout::choose_format;

const NO_OVERLAY_CURSOR: i32 = 0;

/// Captures one fresh frame of the output named `name`.
pub(crate) fn capture_output(
    connection: &Connection,
    name: &str,
    deadline: Deadline,
) -> Result<RawFrame, CaptureFault> {
    let mut capture = CaptureQueue::open(connection, deadline)?;
    let chosen =
        capture_globals::output(&capture.state.globals).map_err(CaptureFault::Unsupported)?;
    let handle = capture.handle();
    let shm: WlShm = capture
        .registry
        .bind(chosen.shm.name, chosen.shm.version, &handle, ());
    let screencopy: ZwlrScreencopyManagerV1 = capture.registry.bind(
        chosen.screencopy.name,
        chosen.screencopy.version,
        &handle,
        (),
    );
    let outputs: Vec<(u32, WlOutput)> = chosen
        .outputs
        .iter()
        .map(|output| {
            let proxy = capture
                .registry
                .bind(output.name, output.version, &handle, output.name);
            (output.name, proxy)
        })
        .collect();
    let outcome = capture
        .sync(deadline)
        .map_err(CaptureFault::Setup)
        .and_then(|()| {
            let output = named(&capture, &outputs, name)?;
            one_frame(&mut capture, &shm, &screencopy, &output, deadline)
        });
    for (_, output) in &outputs {
        output.release();
    }
    screencopy.destroy();
    capture.flush();
    outcome
}

fn named(
    capture: &CaptureQueue,
    outputs: &[(u32, WlOutput)],
    name: &str,
) -> Result<WlOutput, CaptureFault> {
    let mut matching = capture
        .state
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
    capture: &mut CaptureQueue,
    shm: &WlShm,
    screencopy: &ZwlrScreencopyManagerV1,
    output: &WlOutput,
    deadline: Deadline,
) -> Result<RawFrame, CaptureFault> {
    let handle = capture.handle();
    let frame = screencopy.capture_output(NO_OVERLAY_CURSOR, output, &handle, ());
    let announced = capture.wait(
        |state| state.screencopy.offers_done || state.frame.outcome != FrameOutcome::Pending,
        deadline,
    );
    let outcome = announced
        .map_err(CaptureFault::Setup)
        .and_then(|()| copy_into(capture, shm, &frame, deadline));
    frame.destroy();
    outcome
}

fn copy_into(
    capture: &mut CaptureQueue,
    shm: &WlShm,
    frame: &wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_frame_v1::ZwlrScreencopyFrameV1,
    deadline: Deadline,
) -> Result<RawFrame, CaptureFault> {
    if let FrameOutcome::Failed(failure) = capture.state.frame.outcome {
        return Err(CaptureFault::Failed(failure));
    }
    let layout = offered(&capture.state.screencopy.offers).ok_or(CaptureFault::NoFormat)?;
    let buffer = ShmBuffer::create(shm, layout, &capture.handle()).ok_or(CaptureFault::NoFormat)?;
    let requested = monotonic_now();
    frame.copy(buffer.buffer());
    let waited = capture.wait(
        |state| state.frame.outcome != FrameOutcome::Pending,
        deadline,
    );
    settle_frame(waited, capture.state.frame, requested)?;
    Ok(RawFrame {
        data: buffer
            .read()
            .ok_or(CaptureFault::Failed(FrameFailure::Unknown))?,
        format: layout.format,
        width: layout.width,
        height: layout.height,
        stride: layout.stride,
        inverted: capture.state.screencopy.inverted,
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
