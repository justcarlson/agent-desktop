#![expect(dead_code, reason = "stub the owning lane replaces")]

use agent_desktop_core::{AdapterError, Deadline};

use super::SharedBus;
use super::listed_window::ListedWindow;
use super::registry::Registry;
use super::registry_app::RegistryApp;
use super::registry_frames::RootRead;
use super::window_ops::windows_of;

pub(crate) struct AppOutcome {
    pub(crate) app: usize,
    pub(crate) read: RootRead,
}

pub(crate) struct Pass {
    pub(crate) windows: Vec<ListedWindow>,
    pub(crate) outcomes: Vec<AppOutcome>,
    pub(crate) candidates: Vec<usize>,
    pub(crate) focused: Option<usize>,
}

pub(crate) fn run_pass(
    bus: &SharedBus,
    registry: &Registry,
    select: impl Fn(&RegistryApp) -> bool,
    deadline: Deadline,
) -> Result<Pass, AdapterError> {
    Ok(Pass {
        windows: windows_of(bus, registry, select, deadline)?,
        outcomes: Vec::new(),
        candidates: Vec::new(),
        focused: None,
    })
}

#[cfg(test)]
#[path = "signal_inventory_tests.rs"]
mod tests;
