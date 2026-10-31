/// What a screen capture hides before its frame is requested and shows
/// again after, such as the agent's own cursor overlay. A failed `hide`
/// skips the capture; `show` runs whatever the capture returned.
pub(crate) trait CursorHider {
    fn hide(&mut self) -> Result<(), agent_desktop_core::AdapterError>;
    fn show(&mut self) -> Result<(), agent_desktop_core::AdapterError>;
}
