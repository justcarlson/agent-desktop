use agent_desktop_core::AdapterError;
use zbus::Connection;

use super::classify;

const BUS_DAEMON: &str = "org.freedesktop.DBus";
const BUS_DAEMON_PATH: &str = "/org/freedesktop/DBus";

/// The process id behind a unique bus name, asked of the bus daemon, which
/// answers even for a process that has stopped answering its own calls.
pub(crate) async fn process_id(
    connection: &Connection,
    bus_name: String,
) -> (String, Result<u32, AdapterError>) {
    let pid = connection
        .call_method(
            Some(BUS_DAEMON),
            BUS_DAEMON_PATH,
            Some(BUS_DAEMON),
            "GetConnectionUnixProcessID",
            &(bus_name.as_str(),),
        )
        .await
        .and_then(|reply| reply.body().deserialize::<u32>())
        .map_err(|error| classify(&error));
    (bus_name, pid)
}
