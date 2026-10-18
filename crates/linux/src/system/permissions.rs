use agent_desktop_core::{AdapterError, Deadline, PermissionReport, PermissionState};

use crate::system::SharedBus;
use crate::system::bus::{ensure_budget, probe_registry};

pub(crate) fn report(
    bus: &SharedBus,
    deadline: Deadline,
) -> Result<PermissionReport, AdapterError> {
    let connection = bus.get(deadline)?;
    probe_registry(&connection, deadline)?;
    ensure_budget(deadline)?;
    Ok(PermissionReport {
        accessibility: PermissionState::Granted,
        screen_recording: PermissionState::Unknown,
        automation: PermissionState::NotRequired,
    })
}
