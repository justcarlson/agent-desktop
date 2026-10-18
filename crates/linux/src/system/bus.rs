use std::io::ErrorKind;
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use agent_desktop_core::{AdapterError, DEFAULT_OPERATION_TIMEOUT_MS, Deadline, ErrorCode};
use zbus::Connection as AsyncConnection;
use zbus::blocking::Connection;
use zbus::connection::Builder;

use super::bus_deadline::within;
use super::bus_fault::BusFault;
use super::bus_stage::Stage;

const BUS_SERVICE: &str = "org.a11y.Bus";
const BUS_PATH: &str = "/org/a11y/bus";
pub(crate) const REGISTRY_SERVICE: &str = "org.a11y.atspi.Registry";
pub(crate) const REGISTRY_ROOT_PATH: &str = "/org/a11y/atspi/accessible/root";
pub(crate) const PROPERTIES_INTERFACE: &str = "org.freedesktop.DBus.Properties";
pub(crate) const ACCESSIBLE_INTERFACE: &str = "org.a11y.atspi.Accessible";

pub(crate) fn ensure_budget(deadline: Deadline) -> Result<(), AdapterError> {
    if deadline.is_expired() {
        Err(deadline.timeout_error())
    } else {
        Ok(())
    }
}

pub(crate) fn classify(error: &zbus::Error) -> AdapterError {
    classify_at(Stage::AccessibilityBus, error)
}

pub(crate) fn classify_at(stage: Stage, error: &zbus::Error) -> AdapterError {
    if let zbus::Error::InputOutput(io) | zbus::Error::Connection(io, _) = error
        && io.kind() == ErrorKind::TimedOut
    {
        return AdapterError::timeout("The accessibility bus did not answer in time")
            .with_platform_detail(error.to_string());
    }
    match BusFault::of(stage, error) {
        Some(fault) => fault.into_error(error),
        None => AdapterError::new(
            ErrorCode::Internal,
            "The accessibility bus returned an unexpected error",
        )
        .with_suggestion("Retry the command; report the platform detail if it persists")
        .with_platform_detail(error.to_string()),
    }
}

fn discover_address(deadline: Deadline) -> Result<String, AdapterError> {
    within(deadline, async {
        let session = Builder::session()
            .map_err(|error| classify_at(Stage::SessionBus, &error))?
            .build()
            .await
            .map_err(|error| classify_at(Stage::SessionBus, &error))?;
        let reply = session
            .call_method(
                Some(BUS_SERVICE),
                BUS_PATH,
                Some(BUS_SERVICE),
                "GetAddress",
                &(),
            )
            .await
            .map_err(|error| classify_at(Stage::SessionBus, &error))?;
        reply
            .body()
            .deserialize::<String>()
            .map_err(|error| classify_at(Stage::SessionBus, &error))
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
/// no application is registered. A connection-level failure drops `connection`
/// from `bus` so the next call reconnects.
pub(crate) fn probe_registry(
    bus: &SharedBus,
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
            .map_err(|error| {
                if is_connection_failure(&error) {
                    bus.forget(connection);
                }
                classify(&error)
            })
    })?;
    ensure_budget(deadline)
}

pub(crate) fn is_connection_failure(error: &zbus::Error) -> bool {
    match error {
        zbus::Error::InputOutput(io) => io.kind() != ErrorKind::TimedOut,
        zbus::Error::MethodError(..) => false,
        _ => false,
    }
}

/// The process-wide accessibility connection, opened on first use. A success
/// is kept; a failure is returned and the next call tries again.
#[derive(Default)]
pub(crate) struct SharedBus {
    slot: Mutex<Option<Connection>>,
}

impl SharedBus {
    fn cached(&self) -> Option<Connection> {
        self.slot
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    pub(crate) fn get(&self, deadline: Deadline) -> Result<Connection, AdapterError> {
        ensure_budget(deadline)?;
        if let Some(connection) = self.cached() {
            return Ok(connection);
        }
        let connection = connect(deadline)?;
        let mut slot = self.slot.lock().unwrap_or_else(PoisonError::into_inner);
        Ok(slot.get_or_insert(connection).clone())
    }

    pub(crate) fn forget(&self, connection: &Connection) {
        let mut slot = self.slot.lock().unwrap_or_else(PoisonError::into_inner);
        let same = slot
            .as_ref()
            .is_some_and(|held| held.inner().unique_name() == connection.inner().unique_name());
        if same {
            *slot = None;
        }
    }
}

#[cfg(test)]
#[path = "bus_tests.rs"]
mod tests;
