use agent_desktop_core::AdapterError;

use super::cursor_hider::CursorHider;

/// The hider for a session that draws nothing of its own over the screen.
pub(crate) struct NoCursorHider;

impl CursorHider for NoCursorHider {
    fn hide(&mut self) -> Result<(), AdapterError> {
        Ok(())
    }

    fn show(&mut self) -> Result<(), AdapterError> {
        Ok(())
    }
}
