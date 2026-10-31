use agent_desktop_core::{AdapterError, WindowInfo};

use super::fake_snapshot::FakeSnapshot;
use super::hypr_world::HyprWorld;
use crate::system::hyprland::clients::desktop_state::DesktopState;
use crate::system::hyprland::clients::hypr_client::HyprClient;
use crate::system::hyprland::toplevel::toplevel_view::ToplevelView;

/// A compositor that reports `before` until it is activated, then `after`
/// once `lag` waits have passed. `budget` is how many waits it allows.
pub(crate) struct FakeHyprWorld {
    pub(crate) clients: Vec<HyprClient>,
    pub(crate) toplevels: Vec<ToplevelView>,
    pub(crate) before: FakeSnapshot,
    pub(crate) after: Option<FakeSnapshot>,
    pub(crate) lag: u32,
    pub(crate) budget: u32,
    pub(crate) live: Option<String>,
    pub(crate) log: Vec<String>,
}

impl FakeHyprWorld {
    fn now(&self) -> &FakeSnapshot {
        let activated = self.log.iter().any(|entry| entry.starts_with("activate"));
        let waited = self.log.iter().filter(|entry| *entry == "wait").count() as u32;
        match &self.after {
            Some(after) if activated && waited >= self.lag => after,
            _ => &self.before,
        }
    }

    pub(crate) fn activations(&self) -> Vec<&String> {
        self.log
            .iter()
            .filter(|entry| entry.starts_with("activate"))
            .collect()
    }
}

impl HyprWorld for FakeHyprWorld {
    fn inventory(&mut self) -> Result<(Vec<HyprClient>, Vec<ToplevelView>), AdapterError> {
        Ok((self.clients.clone(), self.toplevels.clone()))
    }

    fn activated(&mut self) -> Result<Vec<u64>, AdapterError> {
        Ok(self.now().activated.clone())
    }

    fn desktop(&mut self) -> Result<DesktopState, AdapterError> {
        Ok(self.now().desktop.clone())
    }

    fn frame_active(&mut self, _win: &WindowInfo) -> Result<bool, AdapterError> {
        self.log.push("frame".into());
        Ok(self.now().frame_active)
    }

    fn live_instance(&self, _pid: u32) -> Option<String> {
        self.live.clone()
    }

    fn compositor(&self) -> (u32, u64) {
        (50, 60)
    }

    fn activate(&mut self, address: u64) -> Result<(), AdapterError> {
        self.log.push(format!("activate {address:#x}"));
        Ok(())
    }

    fn wait_more(&mut self) -> bool {
        let waited = self.log.iter().filter(|entry| *entry == "wait").count() as u32;
        if waited >= self.budget {
            return false;
        }
        self.log.push("wait".into());
        true
    }
}
