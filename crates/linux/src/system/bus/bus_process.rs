use agent_desktop_core::AdapterError;
use zbus::Connection;

use crate::system::classify;

const BUS_DAEMON: &str = "org.freedesktop.DBus";
const BUS_DAEMON_PATH: &str = "/org/freedesktop/DBus";

/// The process id behind a unique bus name as the bus daemon reports it, with
/// the daemon's own error. A name nobody owns is `NameHasNoOwner`, which means
/// the application left, so a caller that must tell that from an unreachable
/// bus reads the error itself.
pub(crate) async fn owner_process_id(
    connection: &Connection,
    bus_name: &str,
) -> Result<u32, zbus::Error> {
    connection
        .call_method(
            Some(BUS_DAEMON),
            BUS_DAEMON_PATH,
            Some(BUS_DAEMON),
            "GetConnectionUnixProcessID",
            &(bus_name,),
        )
        .await
        .and_then(|reply| reply.body().deserialize::<u32>())
}

/// The process id behind a unique bus name, asked of the bus daemon, which
/// answers even for a process that has stopped answering its own calls.
pub(crate) async fn process_id(
    connection: &Connection,
    bus_name: String,
) -> (String, Result<u32, AdapterError>) {
    let pid = owner_process_id(connection, &bus_name)
        .await
        .map_err(|error| classify(&error));
    (bus_name, pid)
}
