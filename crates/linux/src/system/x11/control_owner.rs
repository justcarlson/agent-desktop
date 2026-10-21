use super::control::ControlError;
use crate::system::liveness::Identity;

pub(super) struct CloseOwner {
    pub(super) pid: u32,
    pub(super) instance: String,
}

impl CloseOwner {
    pub(super) fn send<G>(
        &self,
        grab: impl FnOnce() -> Result<G, ControlError>,
        owner: impl FnOnce() -> Option<u32>,
        identity: impl FnOnce(u32, &str) -> Identity,
        send: impl FnOnce() -> Result<(), ControlError>,
    ) -> Result<(), ControlError> {
        let _grab = grab()?;
        if owner() != Some(self.pid) || identity(self.pid, &self.instance) != Identity::Same {
            return Err(ControlError::StaleWindow);
        }
        send()
    }
}

#[cfg(test)]
#[path = "control_owner_tests.rs"]
mod tests;
