use agent_desktop_core::{AdapterError, Deadline};

/// What a headed command hides before it captures the screen or sends
/// input, and shows again after: the agent's own cursor overlay. An error
/// from `hide` means nothing was delivered, so the caller sends nothing.
/// `show` runs on every exit path once `hide` succeeded.
pub(crate) trait CursorHider {
    fn hide(&mut self, deadline: Deadline) -> Result<(), AdapterError>;
    fn show(&mut self);
}
