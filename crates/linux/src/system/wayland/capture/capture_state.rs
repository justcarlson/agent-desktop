use super::frame_record::FrameRecord;
use super::screencopy_record::ScreencopyRecord;
use crate::system::wayland::exchange::{Exchange, HoldsExchange};

/// Everything one capture link has been told: the registry and its round
/// trips, the screencopy offers, the frame, and the name each bound output
/// announced keyed by its registry name.
#[derive(Debug, Default)]
pub(crate) struct CaptureState {
    pub(crate) exchange: Exchange,
    pub(crate) screencopy: ScreencopyRecord,
    pub(crate) frame: FrameRecord,
    pub(crate) outputs: Vec<(u32, String)>,
}

impl HoldsExchange for CaptureState {
    fn exchange(&self) -> &Exchange {
        &self.exchange
    }

    fn exchange_mut(&mut self) -> &mut Exchange {
        &mut self.exchange
    }
}
