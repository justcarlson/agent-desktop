use agent_desktop_core::{AdapterError, Deadline};

use super::{
    WatcherRead, classify_daemon, classify_watcher, daemon_fact_error, indicator_extension,
    server_name, unreadable, watcher, watcher_fact_error,
};
use crate::system::session_bus::SessionBus as Bus;

pub(crate) fn require_daemon(deadline: Deadline) -> Result<(), AdapterError> {
    let connection = Bus::get(deadline)
        .map_err(|error| unreadable("notification service", &error.message, "daemon"))?;
    let (daemon, server) = classify_daemon(server_name(&connection, deadline));
    daemon_fact_error(&daemon, server.as_deref())
}

pub(crate) fn require_watcher(deadline: Deadline) -> Result<(), AdapterError> {
    let connection = Bus::get(deadline)
        .map_err(|error| unreadable("system tray watcher", &error.message, "watcher"))?;
    let read = watcher(&connection, deadline);
    let extension = matches!(read, Ok(WatcherRead::NoOwner))
        .then(|| indicator_extension(&connection, deadline))
        .flatten();
    watcher_fact_error(&classify_watcher(read, extension))
}
