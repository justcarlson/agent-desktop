//! The live compositor and accessibility bus behind Hyprland focus.

use std::time::Duration;

use agent_desktop_core::{AdapterError, Deadline, WindowInfo};

use super::hypr_world::HyprWorld;
use crate::system::SharedBus;
use crate::system::bus::bus_deadline::within;
use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;
use crate::system::hyprland::clients::client_read::{read_cursor, read_desktop};
use crate::system::hyprland::clients::desktop_state::DesktopState;
use crate::system::hyprland::clients::hypr_client::HyprClient;
use crate::system::hyprland::identity::inventory_read::read_inventory;
use crate::system::hyprland::toplevel::toplevel_session::ToplevelSession;
use crate::system::hyprland::toplevel::toplevel_view::ToplevelView;
use crate::system::window::{compositor_listing, window_filter::frame_state, window_id};
use crate::tree::{AtspiElement, read_node};

/// How long the compositor and the application get to agree after an
/// activation. Chromium and Electron report ACTIVE after their own focus
/// event, which took 1.8 s on an idle box.
const PROOF_BUDGET: Duration = Duration::from_millis(6_000);
const PROOF_POLL: Duration = Duration::from_millis(50);
/// What the last proof read keeps of the command deadline. A read that the
/// deadline cuts short would report a timeout in place of what it saw.
const READ_RESERVE: Duration = Duration::from_millis(250);

/// One bound compositor, its foreign toplevel session and the bus.
pub(crate) struct LiveHyprWorld<'a> {
    binding: HyprlandBinding,
    session: ToplevelSession,
    bus: &'a SharedBus,
    deadline: Deadline,
    proof_until: Option<Deadline>,
}

impl<'a> LiveHyprWorld<'a> {
    pub(crate) fn open(
        binding: HyprlandBinding,
        bus: &'a SharedBus,
        deadline: Deadline,
    ) -> Result<Self, AdapterError> {
        let session = ToplevelSession::open(&binding, deadline)?;
        Ok(Self {
            binding,
            session,
            bus,
            deadline,
            proof_until: None,
        })
    }
}

impl HyprWorld for LiveHyprWorld<'_> {
    fn inventory(&mut self) -> Result<(Vec<HyprClient>, Vec<ToplevelView>), AdapterError> {
        read_inventory(&self.binding, self.deadline)
    }

    fn activated(&mut self) -> Result<Vec<u64>, AdapterError> {
        self.session.settle()?;
        Ok(self
            .session
            .windows()
            .into_iter()
            .filter(|window| window.activated)
            .map(|window| window.address)
            .collect())
    }

    fn desktop(&mut self) -> Result<DesktopState, AdapterError> {
        let mut desktop = read_desktop(&self.binding, self.deadline)?;
        desktop.cursor = Some(read_cursor(&self.binding, self.deadline)?);
        Ok(desktop)
    }

    fn frame_active(&mut self, win: &WindowInfo) -> Result<bool, AdapterError> {
        let (bus_name, path) = window_id::decode(&win.id)?;
        let frame = AtspiElement::new(bus_name, path, win.pid.get());
        let shared = self.bus.get(self.deadline)?;
        let node = within(self.deadline, async {
            read_node(shared.inner(), &frame).await
        })?;
        Ok(frame_state(&node.traits.states).active == Some(true))
    }

    fn live_instance(&self, pid: u32) -> Option<String> {
        compositor_listing::live_instance(pid)
    }

    fn compositor(&self) -> (u32, u64) {
        (self.binding.pid, self.binding.start_time)
    }

    fn activate(&mut self, address: u64) -> Result<(), AdapterError> {
        self.proof_until = Some(
            self.deadline
                .capped(proof_window(self.deadline.remaining())),
        );
        self.session.activate(address)
    }

    fn wait_more(&mut self) -> bool {
        let until = self.proof_until.unwrap_or(self.deadline);
        if until.is_expired() {
            return false;
        }
        std::thread::sleep(PROOF_POLL.min(until.remaining()));
        true
    }
}

fn proof_window(remaining: Duration) -> Duration {
    PROOF_BUDGET.min(remaining.saturating_sub(READ_RESERVE))
}

#[cfg(test)]
#[path = "live_hypr_world_tests.rs"]
mod tests;
