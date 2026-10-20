use agent_desktop_core::{AdapterError, Deadline, ErrorCode};
use zbus::zvariant::OwnedValue;

use super::call;

const WATCHER: &str = "org.kde.StatusNotifierWatcher";
const WATCHER_PATH: &str = "/StatusNotifierWatcher";
const REGISTERED: &str = "RegisteredStatusNotifierItems";

/// The registration strings the watcher lists, as it lists them.
pub(crate) fn registered_items(deadline: Deadline) -> Result<Vec<String>, AdapterError> {
    let connection = call::session(deadline)?;
    read_if_registered(call::unique_owner(&connection, WATCHER, deadline)?, || {
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
    })
}

fn read_if_registered<T>(
    owner: Option<String>,
    read: impl FnOnce() -> Result<T, AdapterError>,
) -> Result<T, AdapterError> {
    require_owner(owner)?;
    read()
}

fn require_owner(owner: Option<String>) -> Result<(), AdapterError> {
    owner.map(|_| ()).ok_or_else(watcher_absent)
}

fn watcher_absent() -> AdapterError {
    AdapterError::new(
        ErrorCode::PlatformNotSupported,
        "No system tray watcher is running in this session",
    )
    .with_suggestion("Enable the AppIndicator extension so tray items register")
    .with_details(serde_json::json!({ "watcher": "absent" }))
}

#[cfg(test)]
#[path = "watcher_tests.rs"]
mod tests;
