/// What a teardown found and did, reported as `details.cursor_renderer`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StopOutcome {
    /// No record: nothing was running for the session.
    NotRunning,
    /// The renderer exited after `disable`.
    Stopped,
    /// The renderer exited after a signal, confirmed through its pidfd.
    Terminated,
    /// The renderer was still alive after SIGKILL, or its exit could not be
    /// observed because its pidfd could not be opened.
    ExitUnconfirmed,
    /// A live renderer of another build did not exit after `disable`; it
    /// was not signalled.
    Incompatible,
    /// The record named a process that is gone or a reused pid; nothing was
    /// signalled.
    Stale,
}

impl StopOutcome {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::NotRunning => "not_running",
            Self::Stopped => "stopped",
            Self::Terminated => "terminated",
            Self::ExitUnconfirmed => "exit_unconfirmed",
            Self::Incompatible => "incompatible",
            Self::Stale => "stale",
        }
    }

    /// Whether the session's renderer is known to be gone.
    pub(crate) const fn is_confirmed_gone(self) -> bool {
        matches!(
            self,
            Self::NotRunning | Self::Stopped | Self::Terminated | Self::Stale
        )
    }
}
