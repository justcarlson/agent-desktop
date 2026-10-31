//! One toplevel through `ext_image_copy_capture_v1`. Each capture opens a
//! new session and asks for two frames: the first makes the toplevel
//! redraw, the second, awaited for a bounded time, carries the redraw. A
//! frame the compositor rejects for its buffer is retried on a new session
//! with the new constraints, a bounded number of times.

use agent_desktop_core::Deadline;
use wayland_client::protocol::wl_shm::WlShm;
use wayland_client::Connection;
use wayland_protocols::ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1;
use wayland_protocols::ext::image_capture_source::v1::client::ext_foreign_toplevel_image_capture_source_manager_v1::ExtForeignToplevelImageCaptureSourceManagerV1;
use wayland_protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_manager_v1::{
    ExtImageCopyCaptureManagerV1, Options,
};
use wayland_protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_session_v1::ExtImageCopyCaptureSessionV1;

use super::buffer_layout::BufferLayout;
use super::capture_fault::CaptureFault;
use super::capture_globals;
use super::capture_queue::CaptureQueue;
use super::frame_failure::FrameFailure;
use super::frame_freshness::monotonic_now;
use super::frame_outcome::FrameOutcome;
use super::frame_record::FrameRecord;
use super::frame_redraw::{REDRAW_SETTLE, redrawn};
use super::frame_settle::settle_frame;
use super::raw_frame::RawFrame;
use super::session_record::SessionRecord;
use super::shm_buffer::ShmBuffer;
use super::shm_layout::{buffer_len, choose_format};

const SESSION_ATTEMPTS: usize = 3;

struct Managers {
    shm: WlShm,
    sources: ExtForeignToplevelImageCaptureSourceManagerV1,
    copy: ExtImageCopyCaptureManagerV1,
}

/// Captures one fresh frame of the toplevel behind `toplevel`.
pub(crate) fn capture_toplevel(
    connection: &Connection,
    toplevel: &ExtForeignToplevelHandleV1,
    deadline: Deadline,
) -> Result<RawFrame, CaptureFault> {
    let mut capture = CaptureQueue::open(connection, deadline)?;
    let chosen =
        capture_globals::toplevel(&capture.state.globals).map_err(CaptureFault::Unsupported)?;
    let handle = capture.handle();
    let managers = Managers {
        shm: capture
            .registry
            .bind(chosen.shm.name, chosen.shm.version, &handle, ()),
        sources: capture
            .registry
            .bind(chosen.sources.name, chosen.sources.version, &handle, ()),
        copy: capture
            .registry
            .bind(chosen.copy.name, chosen.copy.version, &handle, ()),
    };
    let mut outcome = Err(CaptureFault::Failed(FrameFailure::BufferConstraints));
    for _ in 0..SESSION_ATTEMPTS {
        outcome = one_session(&mut capture, &managers, toplevel, deadline);
        if outcome != Err(CaptureFault::Failed(FrameFailure::BufferConstraints)) {
            break;
        }
    }
    managers.copy.destroy();
    managers.sources.destroy();
    capture.flush();
    outcome
}

fn one_session(
    capture: &mut CaptureQueue,
    managers: &Managers,
    toplevel: &ExtForeignToplevelHandleV1,
    deadline: Deadline,
) -> Result<RawFrame, CaptureFault> {
    capture.state.session = SessionRecord::default();
    capture.state.frame = FrameRecord::default();
    let handle = capture.handle();
    let source = managers.sources.create_source(toplevel, &handle, ());
    let session = managers
        .copy
        .create_session(&source, Options::empty(), &handle, ());
    let outcome = one_frame(capture, &managers.shm, &session, deadline);
    session.destroy();
    source.destroy();
    capture.flush();
    outcome
}

fn one_frame(
    capture: &mut CaptureQueue,
    shm: &WlShm,
    session: &ExtImageCopyCaptureSessionV1,
    deadline: Deadline,
) -> Result<RawFrame, CaptureFault> {
    capture
        .wait(
            |state| state.session.done || state.session.stopped,
            deadline,
        )
        .map_err(CaptureFault::Setup)?;
    let announced = &capture.state.session;
    if announced.stopped {
        return Err(CaptureFault::Failed(FrameFailure::Stopped));
    }
    let (width, height) = announced.size.ok_or(CaptureFault::NoFormat)?;
    let format = choose_format(&announced.formats).ok_or(CaptureFault::NoFormat)?;
    let (stride, _) = buffer_len(width, height).ok_or(CaptureFault::NoFormat)?;
    let layout = BufferLayout {
        format,
        width,
        height,
        stride: u32::try_from(stride).map_err(|_| CaptureFault::NoFormat)?,
    };
    let buffer = ShmBuffer::create(shm, layout, &capture.handle()).ok_or(CaptureFault::NoFormat)?;
    let first = take_frame(capture, session, (&buffer, layout), deadline)?;
    let second = take_frame(
        capture,
        session,
        (&buffer, layout),
        deadline.capped(REDRAW_SETTLE),
    );
    redrawn(first, second)
}

fn take_frame(
    capture: &mut CaptureQueue,
    session: &ExtImageCopyCaptureSessionV1,
    (buffer, layout): (&ShmBuffer, BufferLayout),
    deadline: Deadline,
) -> Result<RawFrame, CaptureFault> {
    let extent = (
        i32::try_from(layout.width).map_err(|_| CaptureFault::NoFormat)?,
        i32::try_from(layout.height).map_err(|_| CaptureFault::NoFormat)?,
    );
    capture.state.frame = FrameRecord::default();
    let frame = session.create_frame(&capture.handle(), ());
    frame.attach_buffer(buffer.buffer());
    frame.damage_buffer(0, 0, extent.0, extent.1);
    let requested = monotonic_now();
    frame.capture();
    let waited = capture.wait(
        |state| state.frame.outcome != FrameOutcome::Pending,
        deadline,
    );
    frame.destroy();
    settle_frame(waited, capture.state.frame, requested)?;
    Ok(RawFrame {
        data: buffer
            .read()
            .ok_or(CaptureFault::Failed(FrameFailure::Unknown))?,
        format: layout.format,
        width: layout.width,
        height: layout.height,
        stride: layout.stride,
        inverted: false,
    })
}
