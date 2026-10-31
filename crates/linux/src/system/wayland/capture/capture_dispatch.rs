//! The event handlers of a capture queue. Each handler only records what
//! the compositor said; the capture decides what it means.

use std::time::Duration;

use wayland_client::protocol::{wl_buffer, wl_callback, wl_output, wl_registry, wl_shm, wl_shm_pool};
use wayland_client::{Connection, Dispatch, QueueHandle, WEnum};
use wayland_protocols::ext::image_capture_source::v1::client::ext_foreign_toplevel_image_capture_source_manager_v1::ExtForeignToplevelImageCaptureSourceManagerV1;
use wayland_protocols::ext::image_capture_source::v1::client::ext_image_capture_source_v1::ExtImageCaptureSourceV1;
use wayland_protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_frame_v1::{
    self as copy_frame, ExtImageCopyCaptureFrameV1,
};
use wayland_protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_manager_v1::ExtImageCopyCaptureManagerV1;
use wayland_protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_session_v1::{
    self as copy_session, ExtImageCopyCaptureSessionV1,
};
use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_frame_v1::{
    self as screencopy_frame, ZwlrScreencopyFrameV1,
};
use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1;

use super::buffer_layout::BufferLayout;
use super::capture_state::CaptureState;
use super::frame_failure::FrameFailure;
use super::frame_outcome::FrameOutcome;
use crate::system::wayland::toplevel::advertised_global::AdvertisedGlobal;

const Y_INVERT: u32 = 1;

impl Dispatch<wl_registry::WlRegistry, ()> for CaptureState {
    fn event(
        state: &mut Self,
        _: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            state.globals.push(AdvertisedGlobal {
                name,
                interface,
                version,
            });
        }
    }
}

impl Dispatch<wl_callback::WlCallback, ()> for CaptureState {
    fn event(
        state: &mut Self,
        _: &wl_callback::WlCallback,
        event: wl_callback::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_callback::Event::Done { .. } = event {
            state.synced += 1;
        }
    }
}

impl Dispatch<wl_output::WlOutput, u32> for CaptureState {
    fn event(
        state: &mut Self,
        _: &wl_output::WlOutput,
        event: wl_output::Event,
        registry_name: &u32,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_output::Event::Name { name } = event {
            state.outputs.push((*registry_name, name));
        }
    }
}

impl Dispatch<ExtImageCopyCaptureSessionV1, ()> for CaptureState {
    fn event(
        state: &mut Self,
        _: &ExtImageCopyCaptureSessionV1,
        event: copy_session::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let session = &mut state.session;
        match event {
            copy_session::Event::BufferSize { width, height } => {
                session.size = Some((width, height));
            }
            copy_session::Event::ShmFormat {
                format: WEnum::Value(format),
            } => session.formats.push(format.into()),
            copy_session::Event::Done => session.done = true,
            copy_session::Event::Stopped => session.stopped = true,
            _ => {}
        }
    }
}

impl Dispatch<ExtImageCopyCaptureFrameV1, ()> for CaptureState {
    fn event(
        state: &mut Self,
        _: &ExtImageCopyCaptureFrameV1,
        event: copy_frame::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            copy_frame::Event::PresentationTime {
                tv_sec_hi,
                tv_sec_lo,
                tv_nsec,
            } => state.frame.presented = Some(stamp(tv_sec_hi, tv_sec_lo, tv_nsec)),
            copy_frame::Event::Ready => state.frame.outcome = FrameOutcome::Ready,
            copy_frame::Event::Failed { reason } => {
                let failure = match reason {
                    WEnum::Value(copy_frame::FailureReason::BufferConstraints) => {
                        FrameFailure::BufferConstraints
                    }
                    WEnum::Value(copy_frame::FailureReason::Stopped) => FrameFailure::Stopped,
                    _ => FrameFailure::Unknown,
                };
                state.frame.outcome = FrameOutcome::Failed(failure);
            }
            _ => {}
        }
    }
}

impl Dispatch<ZwlrScreencopyFrameV1, ()> for CaptureState {
    fn event(
        state: &mut Self,
        _: &ZwlrScreencopyFrameV1,
        event: screencopy_frame::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            screencopy_frame::Event::Buffer {
                format: WEnum::Value(format),
                width,
                height,
                stride,
            } => state.screencopy.offers.push(BufferLayout {
                format: format.into(),
                width,
                height,
                stride,
            }),
            screencopy_frame::Event::BufferDone => state.screencopy.offers_done = true,
            screencopy_frame::Event::Flags {
                flags: WEnum::Value(flags),
            } => state.screencopy.inverted = flags.bits() & Y_INVERT != 0,
            screencopy_frame::Event::Ready {
                tv_sec_hi,
                tv_sec_lo,
                tv_nsec,
            } => {
                state.frame.presented = Some(stamp(tv_sec_hi, tv_sec_lo, tv_nsec));
                state.frame.outcome = FrameOutcome::Ready;
            }
            screencopy_frame::Event::Failed => {
                state.frame.outcome = FrameOutcome::Failed(FrameFailure::Unknown);
            }
            _ => {}
        }
    }
}

macro_rules! silent {
    ($($proxy:ty),* $(,)?) => {
        $(
            impl Dispatch<$proxy, ()> for CaptureState {
                fn event(
                    _: &mut Self,
                    _: &$proxy,
                    _: <$proxy as wayland_client::Proxy>::Event,
                    _: &(),
                    _: &Connection,
                    _: &QueueHandle<Self>,
                ) {
                }
            }
        )*
    };
}

silent!(
    wl_shm::WlShm,
    wl_shm_pool::WlShmPool,
    wl_buffer::WlBuffer,
    ExtForeignToplevelImageCaptureSourceManagerV1,
    ExtImageCaptureSourceV1,
    ExtImageCopyCaptureManagerV1,
    ZwlrScreencopyManagerV1,
);

/// A protocol timestamp split into high and low seconds and nanoseconds.
pub(crate) fn stamp(seconds_high: u32, seconds_low: u32, nanoseconds: u32) -> Duration {
    let seconds = (u64::from(seconds_high) << 32) | u64::from(seconds_low);
    Duration::new(seconds, nanoseconds.min(999_999_999))
}

#[cfg(test)]
#[path = "capture_dispatch_tests.rs"]
mod tests;
