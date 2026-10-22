use std::ffi::OsString;
use std::sync::{Mutex, PoisonError};

use agent_desktop_core::{AdapterError, Deadline};
use zbus::Connection;

pub(crate) struct CachedConnection {
    environment: Vec<Option<OsString>>,
    pub(crate) connection: Connection,
}

fn environment() -> Vec<Option<OsString>> {
    [
        "DBUS_SESSION_BUS_ADDRESS",
        "AT_SPI_BUS_ADDRESS",
        "XDG_RUNTIME_DIR",
        "DISPLAY",
        "WAYLAND_DISPLAY",
        "XDG_SESSION_TYPE",
    ]
    .into_iter()
    .map(std::env::var_os)
    .collect()
}

pub(crate) fn get(
    slot: &Mutex<Option<CachedConnection>>,
    deadline: Deadline,
    open: impl FnOnce() -> Result<Connection, AdapterError>,
) -> Result<Connection, AdapterError> {
    crate::system::ensure_budget(deadline)?;
    let scope = environment();
    let mut held = slot.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(cached) = held.as_ref()
        && cached.environment == scope
        && !cached.connection.is_closed()
    {
        return Ok(cached.connection.clone());
    }
    *held = None;
    drop(held);
    let connection = open()?;
    crate::system::ensure_budget(deadline)?;
    let mut held = slot.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(cached) = held.as_ref()
        && cached.environment == scope
        && !cached.connection.is_closed()
    {
        return Ok(cached.connection.clone());
    }
    *held = Some(CachedConnection {
        environment: scope,
        connection: connection.clone(),
    });
    Ok(connection)
}

pub(crate) fn forget(slot: &Mutex<Option<CachedConnection>>, connection: &Connection) {
    let mut held = slot.lock().unwrap_or_else(PoisonError::into_inner);
    if held.as_ref().is_some_and(|cached| {
        cached.connection.server_guid() == connection.server_guid()
            && cached.connection.unique_name() == connection.unique_name()
    }) {
        *held = None;
    }
}

#[cfg(test)]
#[path = "cached_connection_tests.rs"]
mod tests;
