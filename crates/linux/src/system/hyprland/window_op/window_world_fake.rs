use agent_desktop_core::AdapterError;

use super::client_state::ClientState;
use super::config_dialect::DIALECT_PROBE;
use super::window_world::WindowWorld;
use crate::system::hyprland::clients::desktop_state::DesktopState;
use crate::system::hyprland::clients::hypr_client::HyprClient;
use crate::system::hyprland::toplevel::toplevel_view::ToplevelView;

/// A compositor that shows `frames[n]` once `n` dispatch requests were sent,
/// holding at the last frame. It answers the dialect probe with
/// `eval_reply` and every dispatch with `reply`; `budget` is how many
/// read-back waits it allows.
pub(crate) struct FakeWindowWorld {
    pub(crate) clients: Vec<HyprClient>,
    pub(crate) toplevels: Vec<ToplevelView>,
    pub(crate) frames: Vec<(Vec<ClientState>, DesktopState)>,
    pub(crate) eval_reply: String,
    pub(crate) reply: String,
    pub(crate) budget: u32,
    pub(crate) log: Vec<String>,
}

impl FakeWindowWorld {
    pub(crate) fn dispatches(&self) -> Vec<&str> {
        self.log
            .iter()
            .filter(|entry| entry.starts_with("dispatch "))
            .map(String::as_str)
            .collect()
    }

    fn frame(&self) -> &(Vec<ClientState>, DesktopState) {
        let sent = self.dispatches().len();
        &self.frames[sent.min(self.frames.len() - 1)]
    }
}

impl WindowWorld for FakeWindowWorld {
    fn inventory(&mut self) -> Result<(Vec<HyprClient>, Vec<ToplevelView>), AdapterError> {
        Ok((self.clients.clone(), self.toplevels.clone()))
    }

    fn states(&mut self) -> Result<Vec<ClientState>, AdapterError> {
        Ok(self.frame().0.clone())
    }

    fn desktop(&mut self) -> Result<DesktopState, AdapterError> {
        Ok(self.frame().1.clone())
    }

    fn request(&mut self, command: &str) -> Result<String, AdapterError> {
        self.log.push(command.to_owned());
        if command == DIALECT_PROBE {
            return Ok(self.eval_reply.clone());
        }
        Ok(self.reply.clone())
    }

    fn live_instance(&self, _pid: u32) -> Option<String> {
        Some("boot:100".into())
    }

    fn compositor(&self) -> (u32, u64) {
        (50, 60)
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
