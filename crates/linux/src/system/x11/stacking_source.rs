use super::X11State;
use super::stacking_read::StackingRead;

/// Where stacking evidence comes from. The real source talks to the display
/// server; tests supply a fixed answer. `target_client` selects which
/// override-redirect windows count as above.
pub(crate) trait StackingSource: Send + Sync {
    fn read(&self, target_client: u32) -> StackingRead;

    /// The managed clients, for finding the client of a listed window. A
    /// source that cannot list them says so, and no window maps to a client.
    fn clients(&self) -> X11State {
        X11State::Unavailable
    }
}
