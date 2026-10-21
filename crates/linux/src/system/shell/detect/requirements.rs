use agent_desktop_core::{AdapterError, Deadline};

use super::{
    CallFailure, Fact, NOTIFICATIONS, call, classify_daemon, classify_watcher, daemon_fact_error,
    server_name, shell_owner, unreadable, watcher, watcher_fact_error,
};
use crate::system::session_bus::SessionBus as Bus;

pub(crate) fn require_daemon(deadline: Deadline) -> Result<(), AdapterError> {
    let connection = Bus::get(deadline)
        .map_err(|error| unreadable("notification service", &error.message, "daemon"))?;
    let (daemon, server) = classify_daemon(server_name(&connection, deadline));
    daemon_fact_error(&daemon, server.as_deref())?;
    let owner = call(
        deadline,
        crate::system::owner_process_id(&connection, NOTIFICATIONS),
    );
    let backend = call(deadline, shell_owner(&connection));
    match (owner, backend) {
        (Ok(owner), Ok(backend)) if owner == backend => Ok(()),
        (Ok(_), Ok(_) | Err(CallFailure::NoService)) => daemon_fact_error(
            &Fact::Absent("notification and shell service owners differ".to_owned()),
            server.as_deref(),
        ),
        (Err(error), _) | (_, Err(error)) => Err(unreadable(
            "notification service ownership",
            &format!("{error:?}"),
            "daemon",
        )),
    }
}

pub(crate) fn require_watcher(deadline: Deadline) -> Result<(), AdapterError> {
    let connection = Bus::get(deadline)
        .map_err(|error| unreadable("system tray watcher", &error.message, "watcher"))?;
    let read = watcher(&connection, deadline);
    watcher_fact_error(&classify_watcher(read))
}
