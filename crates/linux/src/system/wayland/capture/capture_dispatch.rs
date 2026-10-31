//! The event handlers of a capture link. Each handler only records what
//! the compositor said; the capture decides what it means.

use std::time::Duration;

use wayland_client::protocol::{
    wl_buffer, wl_callback, wl_output, wl_registry, wl_shm, wl_shm_pool,
};
use wayland_client::{Connection, Dispatch, QueueHandle, WEnum, delegate_dispatch, delegate_noop};
use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_frame_v1::{
    self as screencopy_frame, ZwlrScreencopyFrameV1,
};
use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1;

use super::buffer_layout::BufferLayout;
use super::capture_state::CaptureState;
use super::frame_failure::FrameFailure;
use super::frame_outcome::FrameOutcome;
use crate::system::wayland::exchange::ExchangeHandler;

const Y_INVERT: u32 = 1;

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

delegate_dispatch!(CaptureState: [wl_registry::WlRegistry: ()] => ExchangeHandler);
delegate_dispatch!(CaptureState: [wl_callback::WlCallback: u32] => ExchangeHandler);
delegate_noop!(CaptureState: ignore wl_shm::WlShm);
delegate_noop!(CaptureState: ignore wl_shm_pool::WlShmPool);
delegate_noop!(CaptureState: ignore wl_buffer::WlBuffer);
delegate_noop!(CaptureState: ignore ZwlrScreencopyManagerV1);

/// A protocol timestamp split into high and low seconds and nanoseconds.
pub(crate) fn stamp(seconds_high: u32, seconds_low: u32, nanoseconds: u32) -> Duration {
    let seconds = (u64::from(seconds_high) << 32) | u64::from(seconds_low);
    Duration::new(seconds, nanoseconds.min(999_999_999))
}

#[cfg(test)]
#[path = "capture_dispatch_tests.rs"]
mod tests;
