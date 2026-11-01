use std::time::Duration;

/// How long a teardown waits at each step: for exit after `disable`, after
/// SIGTERM and after SIGKILL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StopTimings {
    pub(crate) after_disable: Duration,
    pub(crate) after_term: Duration,
    pub(crate) after_kill: Duration,
}

impl Default for StopTimings {
    fn default() -> Self {
        Self {
            after_disable: Duration::from_secs(1),
            after_term: Duration::from_secs(1),
            after_kill: Duration::from_secs(1),
        }
    }
}
