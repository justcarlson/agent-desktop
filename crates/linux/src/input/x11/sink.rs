use agent_desktop_core::{AdapterError, ErrorCode};

use crate::input::keymap::{Keymap, KeymapEdit};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SinkError {
    Closed,
    Request(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PointerRead {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) mask: u16,
}

impl SinkError {
    pub(crate) fn into_adapter(self) -> AdapterError {
        match self {
            Self::Closed => AdapterError::new(
                ErrorCode::ActionFailed,
                "The display connection closed while input was being sent",
            ),
            Self::Request(detail) => AdapterError::new(
                ErrorCode::ActionFailed,
                "The display server rejected an input request",
            )
            .with_platform_detail(detail),
        }
    }
}

pub(crate) trait X11Sink {
    fn key(&mut self, code: u8, down: bool) -> Result<(), SinkError>;
    fn button(&mut self, code: u8, down: bool) -> Result<(), SinkError>;
    fn move_to(&mut self, x: i16, y: i16) -> Result<(), SinkError>;
    fn round_trip(&mut self) -> Result<(), SinkError>;
    fn pointer(&mut self) -> Result<PointerRead, SinkError>;
    fn keys_down(&mut self) -> Result<[u8; 32], SinkError>;
    fn keymap(&mut self) -> Result<Keymap, SinkError>;
    fn edit_keymap(&mut self, edit: &KeymapEdit) -> Result<(), SinkError>;
    fn root_size(&mut self) -> Result<(u16, u16), SinkError>;
}

#[cfg(test)]
#[path = "sink_tests.rs"]
mod tests;
