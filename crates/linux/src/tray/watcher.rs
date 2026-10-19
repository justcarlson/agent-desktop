use agent_desktop_core::{AdapterError, Deadline};
use zbus::zvariant::OwnedValue;

use super::call;

const WATCHER: &str = "org.kde.StatusNotifierWatcher";
const WATCHER_PATH: &str = "/StatusNotifierWatcher";
const REGISTERED: &str = "RegisteredStatusNotifierItems";

/// The registration strings the watcher lists, as it lists them. A session
/// with no watcher on the bus registers nothing, so the list is empty.
pub(crate) fn registered_items(deadline: Deadline) -> Result<Vec<String>, AdapterError> {
    let connection = call::session(deadline)?;
    if call::unique_owner(&connection, WATCHER, deadline)?.is_none() {
        return Ok(Vec::new());
    }
    let reply = call::call(
        &connection,
        call::Endpoint {
            destination: WATCHER,
            path: WATCHER_PATH,
            interface: "org.freedesktop.DBus.Properties",
        },
        "Get",
        &(WATCHER, REGISTERED),
        deadline,
    )?;
    let value: OwnedValue = reply
        .body()
        .deserialize()
        .map_err(|error| call::fault(&error))?;
    Vec::<String>::try_from(value).map_err(|error| {
        AdapterError::internal("The tray watcher's item list could not be read")
            .with_platform_detail(error.to_string())
    })
}
