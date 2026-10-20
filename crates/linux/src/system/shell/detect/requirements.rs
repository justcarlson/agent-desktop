use agent_desktop_core::{AdapterError, Deadline};

use super::{LiveSession, SessionCalls, classify_daemon, daemon_fact_error, unreadable};
use crate::system::session_bus::SessionBus;

pub(super) fn require_daemon_with<S: SessionCalls>(session: &S) -> Result<(), AdapterError> {
    let (daemon, server) = classify_daemon(session.server_name());
    daemon_fact_error(&daemon, server.as_deref())
}

pub(crate) fn require_daemon(deadline: Deadline) -> Result<(), AdapterError> {
    let connection = SessionBus::get(deadline)
        .map_err(|error| unreadable("notification service", &error.message, "daemon"))?;
    let session = LiveSession {
        connection,
        deadline,
    };
    require_daemon_with(&session)
}
