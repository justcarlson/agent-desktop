use agent_desktop_core::{AdapterError, Deadline};

use super::{
    LiveSession, SessionCalls, WatcherRead, classify_daemon, classify_watcher, daemon_fact_error,
    unreadable, watcher_fact_error,
};
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

pub(super) fn require_watcher_with<S: SessionCalls>(session: &S) -> Result<(), AdapterError> {
    let read = session.watcher();
    let extension = matches!(read, Ok(WatcherRead::NoOwner))
        .then(|| session.indicator_extension())
        .flatten();
    watcher_fact_error(&classify_watcher(read, extension))
}

pub(crate) fn require_watcher(deadline: Deadline) -> Result<(), AdapterError> {
    let connection = SessionBus::get(deadline)
        .map_err(|error| unreadable("system tray watcher", &error.message, "watcher"))?;
    let session = LiveSession {
        connection,
        deadline,
    };
    require_watcher_with(&session)
}
