use std::sync::Arc;
use std::time::Duration;

use agent_desktop_core::{AdapterError, ErrorCode};

use super::output_point::OutputPoint;
use super::pointer_sink::PointerSink;
use super::wheel_axis::WheelAxis;
use crate::input::x11::progress::Progress;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SinkEvent {
    Motion(OutputPoint),
    Button(u32, bool),
    Wheel(WheelAxis, i32),
    Recover(Vec<u32>),
}

#[derive(Default)]
pub(crate) struct FakePointerSink {
    pub(crate) events: Vec<SinkEvent>,
    pub(crate) fail_at: Option<usize>,
    pub(crate) cancel_after: Option<(usize, Arc<Progress>)>,
}

impl FakePointerSink {
    fn record(&mut self, event: SinkEvent) -> Result<(), AdapterError> {
        if self.fail_at == Some(self.events.len()) {
            return Err(AdapterError::new(ErrorCode::ActionFailed, "sink broke"));
        }
        self.events.push(event);
        if let Some((count, progress)) = &self.cancel_after
            && *count == self.events.len()
        {
            progress.cancel();
        }
        Ok(())
    }
}

impl PointerSink for FakePointerSink {
    fn motion(&mut self, point: &OutputPoint) -> Result<(), AdapterError> {
        self.record(SinkEvent::Motion(point.clone()))
    }

    fn button(&mut self, code: u32, down: bool) -> Result<(), AdapterError> {
        self.record(SinkEvent::Button(code, down))
    }

    fn wheel(&mut self, axis: WheelAxis, discrete: i32) -> Result<(), AdapterError> {
        self.record(SinkEvent::Wheel(axis, discrete))
    }

    fn pause(&mut self, _duration: Duration) {}

    fn recover(&mut self, buttons: &[u32]) -> bool {
        self.events.push(SinkEvent::Recover(buttons.to_vec()));
        true
    }
}
