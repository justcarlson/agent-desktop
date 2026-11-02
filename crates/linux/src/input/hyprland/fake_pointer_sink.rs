use std::sync::Arc;
use std::time::Duration;

use agent_desktop_core::{AdapterError, ErrorCode};

use super::output_point::OutputPoint;
use super::pointer_sink::PointerSink;
use super::send_failure::SendFailure;
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
    pub(crate) fail_after_send_at: Option<usize>,
    pub(crate) cancel_after: Option<(usize, Arc<Progress>)>,
    pub(crate) finish_error: Option<ErrorCode>,
    pub(crate) finished: usize,
    pub(crate) settled: usize,
}

impl FakePointerSink {
    fn record(&mut self, event: SinkEvent) -> Result<(), SendFailure> {
        if self.fail_at == Some(self.events.len()) {
            return Err(AdapterError::new(ErrorCode::ActionFailed, "sink broke").into());
        }
        self.events.push(event);
        if self.fail_after_send_at == Some(self.events.len() - 1) {
            return Err(SendFailure::after_dispatch(AdapterError::new(
                ErrorCode::ActionFailed,
                "sink broke after sending",
            )));
        }
        if let Some((count, progress)) = &self.cancel_after
            && *count == self.events.len()
        {
            progress.cancel();
        }
        Ok(())
    }
}

impl PointerSink for FakePointerSink {
    fn motion(&mut self, point: &OutputPoint) -> Result<(), SendFailure> {
        self.record(SinkEvent::Motion(point.clone()))
    }

    fn button(&mut self, code: u32, down: bool) -> Result<(), SendFailure> {
        self.record(SinkEvent::Button(code, down))
    }

    fn wheel(&mut self, axis: WheelAxis, discrete: i32) -> Result<(), SendFailure> {
        self.record(SinkEvent::Wheel(axis, discrete))
    }

    fn pause(&mut self, _duration: Duration) {}

    fn recover(&mut self, buttons: &[u32]) -> bool {
        self.events.push(SinkEvent::Recover(buttons.to_vec()));
        true
    }

    fn finish(&mut self) -> Result<(), AdapterError> {
        self.finished += 1;
        match &self.finish_error {
            Some(code) => Err(AdapterError::new(
                code.clone(),
                "the final roundtrip timed out",
            )),
            None => Ok(()),
        }
    }

    fn settle(&mut self) -> bool {
        self.settled += 1;
        true
    }
}
