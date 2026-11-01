//! The live bound compositor behind a Hyprland window operation.

use std::time::Duration;

use agent_desktop_core::{AdapterError, Deadline, DeliverySemantics};

use super::client_state::{ClientState, parse_states};
use super::window_world::WindowWorld;
use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;
use crate::system::hyprland::clients::client_read::{read_cursor, read_desktop};
use crate::system::hyprland::clients::desktop_state::DesktopState;
use crate::system::hyprland::clients::hypr_client::HyprClient;
use crate::system::hyprland::identity::inventory_read::read_inventory;
use crate::system::hyprland::ipc::bound_request::bound_request;
use crate::system::hyprland::toplevel::toplevel_view::ToplevelView;
use crate::system::window::compositor_listing;

const CLIENTS_COMMAND: &str = "j/clients";
const DISPATCH_PREFIX: &str = "dispatch ";
/// How long the compositor gets to show a dispatch in `j/clients`. The
/// probes read every effect within 0.6 s.
const SETTLE_BUDGET: Duration = Duration::from_millis(1_000);
const SETTLE_POLL: Duration = Duration::from_millis(25);
/// What the last read-back keeps of the command deadline.
const READ_RESERVE: Duration = Duration::from_millis(250);

/// One bound compositor and the command deadline.
pub(crate) struct LiveWindowWorld {
    binding: HyprlandBinding,
    deadline: Deadline,
    settle_until: Option<Deadline>,
}

impl LiveWindowWorld {
    pub(crate) fn new(binding: HyprlandBinding, deadline: Deadline) -> Self {
        Self {
            binding,
            deadline,
            settle_until: None,
        }
    }
}

impl WindowWorld for LiveWindowWorld {
    fn inventory(&mut self) -> Result<(Vec<HyprClient>, Vec<ToplevelView>), AdapterError> {
        read_inventory(&self.binding, self.deadline)
    }

    fn states(&mut self) -> Result<Vec<ClientState>, AdapterError> {
        let body = bound_request(&self.binding, CLIENTS_COMMAND, self.deadline)?;
        parse_states(&body).ok_or_else(|| {
            AdapterError::internal("The Hyprland client list could not be read")
                .with_suggestion("Retry once the compositor answers")
        })
    }

    fn desktop(&mut self) -> Result<DesktopState, AdapterError> {
        let mut desktop = read_desktop(&self.binding, self.deadline)?;
        desktop.cursor = Some(read_cursor(&self.binding, self.deadline)?);
        Ok(desktop)
    }

    fn request(&mut self, command: &str) -> Result<String, AdapterError> {
        let dispatch = command.starts_with(DISPATCH_PREFIX);
        if dispatch && self.deadline.is_expired() {
            return Err(self
                .deadline
                .timeout_error()
                .with_disposition(DeliverySemantics::not_delivered()));
        }
        let body = bound_request(&self.binding, command, self.deadline)?;
        if dispatch {
            let window = SETTLE_BUDGET.min(self.deadline.remaining().saturating_sub(READ_RESERVE));
            self.settle_until = Some(self.deadline.capped(window));
        }
        Ok(String::from_utf8_lossy(&body).into_owned())
    }

    fn live_instance(&self, pid: u32) -> Option<String> {
        compositor_listing::live_instance(pid)
    }

    fn compositor(&self) -> (u32, u64) {
        (self.binding.pid, self.binding.start_time)
    }

    fn wait_more(&mut self) -> bool {
        let until = self.settle_until.unwrap_or(self.deadline);
        if until.is_expired() {
            return false;
        }
        std::thread::sleep(SETTLE_POLL.min(until.remaining()));
        true
    }
}
