use agent_desktop_core::{AdapterError, Deadline, ErrorCode, PermissionReport, PermissionState};

use crate::system::SharedBus;
use crate::system::bus::{ensure_budget, probe_registry};

pub(crate) fn report(
    bus: &SharedBus,
    deadline: Deadline,
) -> Result<PermissionReport, AdapterError> {
    let probed = bus
        .get(deadline)
        .and_then(|connection| probe_registry(bus, &connection, deadline));
    let report = report_from_probe(probed)?;
    ensure_budget(deadline)?;
    Ok(report)
}

/// An unreachable bus is the accessibility permission being unavailable, so it
/// is reported as `Denied` with the cause-specific recovery step. A timeout or
/// an unexpected bus error stays an error: nothing was learned about the bus.
pub(crate) fn report_from_probe(
    probed: Result<(), AdapterError>,
) -> Result<PermissionReport, AdapterError> {
    let accessibility = match probed {
        Ok(()) => PermissionState::Granted,
        Err(error) if error.code == ErrorCode::PlatformNotSupported => PermissionState::Denied {
            suggestion: error
                .suggestion
                .unwrap_or_else(|| "Start the AT-SPI accessibility bus and retry".into()),
        },
        Err(error) => return Err(error),
    };
    Ok(PermissionReport {
        accessibility,
        screen_recording: PermissionState::Unknown,
        automation: PermissionState::NotRequired,
    })
}

#[cfg(test)]
#[path = "permissions_tests.rs"]
mod tests;
