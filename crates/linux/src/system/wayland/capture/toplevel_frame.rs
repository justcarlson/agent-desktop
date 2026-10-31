//! One toplevel through `ext_image_copy_capture_v1`. Each frame is the
//! first frame of its own session; `frame_redraw` decides, from how the
//! compositor composes the toplevel, which frame is the answer and when a
//! refused buffer is retried. The capture opens its own queue on the
//! connection that holds the foreign toplevel handle, because a handle can
//! be passed only to requests on its own connection.

use agent_desktop_core::Deadline;
use wayland_client::Connection;
use wayland_client::protocol::wl_shm::WlShm;
use wayland_protocols::ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1;
use wayland_protocols::ext::image_capture_source::v1::client::ext_foreign_toplevel_image_capture_source_manager_v1::ExtForeignToplevelImageCaptureSourceManagerV1;
use wayland_protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_manager_v1::{
    ExtImageCopyCaptureManagerV1, Options,
};
use wayland_protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_session_v1::ExtImageCopyCaptureSessionV1;

use super::buffer_layout::BufferLayout;
use super::capture_fault::CaptureFault;
use super::capture_globals::{self, COPY, SHM, SOURCES};
use super::capture_state::CaptureState;
use super::composition::Composition;
use super::frame_failure::FrameFailure;
use super::frame_freshness::monotonic_now;
use super::frame_outcome::FrameOutcome;
use super::frame_record::FrameRecord;
use super::frame_redraw::capture_fresh;
use super::frame_settle::settle_frame;
use super::raw_frame::RawFrame;
use super::session_record::SessionRecord;
use super::shm_buffer::ShmBuffer;
use super::shm_layout::{buffer_len, choose_format};
use crate::system::wayland::link::Link;

type CaptureLink = Link<CaptureState>;

struct Managers {
    shm: WlShm,
    sources: ExtForeignToplevelImageCaptureSourceManagerV1,
    copy: ExtImageCopyCaptureManagerV1,
}

/// Captures one fresh frame of the toplevel behind `toplevel`, a handle on
/// `connection`, which the compositor composes as `composition` says.
pub(crate) fn capture_toplevel(
    connection: &Connection,
    toplevel: &ExtForeignToplevelHandleV1,
    composition: Composition,
    deadline: Deadline,
) -> Result<RawFrame, CaptureFault> {
    let mut state = CaptureState::default();
    let mut link =
        Link::on(connection.clone(), &mut state, deadline).map_err(CaptureFault::Setup)?;
    let chosen = capture_globals::toplevel(&state.exchange).map_err(CaptureFault::Unsupported)?;
    let managers = Managers {
        shm: link.bind(&chosen.shm, SHM.1, ()),
        sources: link.bind(&chosen.sources, SOURCES.1, ()),
        copy: link.bind(&chosen.copy, COPY.1, ()),
    };
    let outcome = capture_fresh(
        deadline,
        composition,
        |_| one_session(&mut link, &mut state, &managers, toplevel),
        std::thread::sleep,
    );
    managers.copy.destroy();
    managers.sources.destroy();
    let _ = link.flush();
    outcome
}

fn one_session(
    link: &mut CaptureLink,
    state: &mut CaptureState,
    managers: &Managers,
    toplevel: &ExtForeignToplevelHandleV1,
) -> Result<RawFrame, CaptureFault> {
    state.session = SessionRecord::default();
    state.frame = FrameRecord::default();
    let handle = link.handle();
    let source = managers.sources.create_source(toplevel, &handle, ());
    let session = managers
        .copy
        .create_session(&source, Options::empty(), &handle, ());
    let outcome = one_frame(link, state, &managers.shm, &session);
    session.destroy();
    source.destroy();
    let _ = link.flush();
    outcome
}

fn one_frame(
    link: &mut CaptureLink,
    state: &mut CaptureState,
    shm: &WlShm,
    session: &ExtImageCopyCaptureSessionV1,
) -> Result<RawFrame, CaptureFault> {
    link.wait(state, |state| state.session.done || state.session.stopped)
        .map_err(CaptureFault::Setup)?;
    let announced = &state.session;
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
    let buffer = ShmBuffer::create(shm, layout, &link.handle()).ok_or(CaptureFault::NoFormat)?;
    take_frame(link, state, session, (&buffer, layout))
}

fn take_frame(
    link: &mut CaptureLink,
    state: &mut CaptureState,
    session: &ExtImageCopyCaptureSessionV1,
    (buffer, layout): (&ShmBuffer, BufferLayout),
) -> Result<RawFrame, CaptureFault> {
    let extent = (
        i32::try_from(layout.width).map_err(|_| CaptureFault::NoFormat)?,
        i32::try_from(layout.height).map_err(|_| CaptureFault::NoFormat)?,
    );
    state.frame = FrameRecord::default();
    let frame = session.create_frame(&link.handle(), ());
    frame.attach_buffer(buffer.buffer());
    frame.damage_buffer(0, 0, extent.0, extent.1);
    let requested = monotonic_now();
    frame.capture();
    let waited = link.wait(state, |state| state.frame.outcome != FrameOutcome::Pending);
    frame.destroy();
    settle_frame(waited, state.frame, requested)?;
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
