use agent_desktop_core::{AdapterError, Deadline};

/// One compositor keyboard session. The calls are the steps of a delivery in
/// the order a caller makes them; `stop` ends whatever `create` made.
pub(crate) trait KeySession {
    fn create(&mut self, deadline: Deadline) -> Result<(), AdapterError>;
    fn start(&mut self, deadline: Deadline) -> Result<(), AdapterError>;
    fn keysym(&mut self, keysym: u32, down: bool, deadline: Deadline) -> Result<(), AdapterError>;
    fn stop(&mut self, deadline: Deadline) -> Result<(), AdapterError>;
}
