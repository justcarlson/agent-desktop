use agent_desktop_core::AdapterError;

/// A pointer event the sink failed to send. `dispatched` says the sink had
/// already queued or written the event, so the compositor may still read it.
#[derive(Debug)]
pub(crate) struct SendFailure {
    pub(crate) error: AdapterError,
    pub(crate) dispatched: bool,
}

impl SendFailure {
    /// A failure after the event was queued or written.
    pub(crate) fn after_dispatch(error: AdapterError) -> Self {
        Self {
            error,
            dispatched: true,
        }
    }
}

impl From<AdapterError> for SendFailure {
    fn from(error: AdapterError) -> Self {
        Self {
            error,
            dispatched: false,
        }
    }
}
