use std::io::ErrorKind;
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use agent_desktop_core::{AdapterError, DEFAULT_OPERATION_TIMEOUT_MS, Deadline, ErrorCode};
use zbus::Connection as AsyncConnection;
use zbus::blocking::Connection;
use zbus::connection::Builder;

use super::bus_deadline::within;

const BUS_SERVICE: &str = "org.a11y.Bus";
const BUS_PATH: &str = "/org/a11y/bus";
const REGISTRY_SERVICE: &str = "org.a11y.atspi.Registry";
const REGISTRY_ROOT_PATH: &str = "/org/a11y/atspi/accessible/root";
const PROPERTIES_INTERFACE: &str = "org.freedesktop.DBus.Properties";
const ACCESSIBLE_INTERFACE: &str = "org.a11y.atspi.Accessible";
const SERVICE_UNKNOWN: &str = "org.freedesktop.DBus.Error.ServiceUnknown";
const NAME_HAS_NO_OWNER: &str = "org.freedesktop.DBus.Error.NameHasNoOwner";
const SPAWN_ERROR_PREFIX: &str = "org.freedesktop.DBus.Error.Spawn.";

const UNREACHABLE_SUGGESTION: &str = "No AT-SPI accessibility bus is reachable. Run inside a desktop session with a D-Bus session bus. On GNOME run 'gsettings set org.gnome.desktop.interface toolkit-accessibility true'. Elsewhere install at-spi2-core and run at-spi-bus-launcher. In Flatpak add '--talk-name=org.a11y.Bus'. In snap connect the 'desktop-legacy' and 'unity7' interfaces.";

pub(crate) fn ensure_budget(deadline: Deadline) -> Result<(), AdapterError> {
    if deadline.is_expired() {
        Err(deadline.timeout_error())
    } else {
        Ok(())
    }
}

pub(crate) fn classify(error: &zbus::Error) -> AdapterError {
    match error {
        zbus::Error::InputOutput(io) | zbus::Error::Connection(io, _)
            if io.kind() == ErrorKind::TimedOut =>
        {
            AdapterError::timeout("The accessibility bus did not answer in time")
                .with_platform_detail(error.to_string())
        }
        zbus::Error::InputOutput(io) | zbus::Error::Connection(io, _)
            if matches!(
                io.kind(),
                ErrorKind::NotFound | ErrorKind::ConnectionRefused
            ) =>
        {
            unreachable_bus(error)
        }
        zbus::Error::Address(_) => unreachable_bus(error),
        zbus::Error::MethodError(name, _, _)
            if matches!(name.as_str(), SERVICE_UNKNOWN | NAME_HAS_NO_OWNER)
                || name.as_str().starts_with(SPAWN_ERROR_PREFIX) =>
        {
            unreachable_bus(error)
        }
        _ => AdapterError::new(
            ErrorCode::Internal,
            "The accessibility bus returned an unexpected error",
        )
        .with_suggestion("Retry the command; report the platform detail if it persists")
        .with_platform_detail(error.to_string()),
    }
}

fn unreachable_bus(error: &zbus::Error) -> AdapterError {
    AdapterError::new(
        ErrorCode::PlatformNotSupported,
        "The AT-SPI accessibility bus is not available in this session",
    )
    .with_suggestion(UNREACHABLE_SUGGESTION)
    .with_platform_detail(error.to_string())
}

fn discover_address(deadline: Deadline) -> Result<String, AdapterError> {
    within(deadline, async {
        let session = Builder::session()
            .map_err(|error| classify(&error))?
            .build()
            .await
            .map_err(|error| classify(&error))?;
        let reply = session
            .call_method(
                Some(BUS_SERVICE),
                BUS_PATH,
                Some(BUS_SERVICE),
                "GetAddress",
                &(),
            )
            .await
            .map_err(|error| classify(&error))?;
        reply
            .body()
            .deserialize::<String>()
            .map_err(|error| classify(&error))
    })
}

fn connect(deadline: Deadline) -> Result<Connection, AdapterError> {
    ensure_budget(deadline)?;
    let address = discover_address(deadline)?;
    ensure_budget(deadline)?;
    let connection: AsyncConnection = within(deadline, async {
        Builder::address(address.as_str())
            .map_err(|error| classify(&error))?
            .method_timeout(Duration::from_millis(DEFAULT_OPERATION_TIMEOUT_MS))
            .build()
            .await
            .map_err(|error| classify(&error))
    })?;
    Ok(connection.into())
}

/// Reads `ChildCount` from the AT-SPI registry root, which answers even when
/// no application is registered.
pub(crate) fn probe_registry(
    connection: &Connection,
    deadline: Deadline,
) -> Result<(), AdapterError> {
    ensure_budget(deadline)?;
    within(deadline, async {
        connection
            .inner()
            .call_method(
                Some(REGISTRY_SERVICE),
                REGISTRY_ROOT_PATH,
                Some(PROPERTIES_INTERFACE),
                "Get",
                &(ACCESSIBLE_INTERFACE, "ChildCount"),
            )
            .await
            .map(|_| ())
            .map_err(|error| classify(&error))
    })?;
    ensure_budget(deadline)
}

/// The process-wide accessibility connection, opened on first use. A success
/// is kept; a failure is returned and the next call tries again.
#[derive(Default)]
pub(crate) struct SharedBus {
    slot: Mutex<Option<Connection>>,
}

impl SharedBus {
    pub(crate) fn get(&self, deadline: Deadline) -> Result<Connection, AdapterError> {
        ensure_budget(deadline)?;
        let mut slot = self.slot.lock().unwrap_or_else(PoisonError::into_inner);
        ensure_budget(deadline)?;
        if let Some(connection) = slot.as_ref() {
            return Ok(connection.clone());
        }
        let connection = connect(deadline)?;
        *slot = Some(connection.clone());
        Ok(connection)
    }
}

#[cfg(test)]
#[path = "bus_tests.rs"]
mod tests;
