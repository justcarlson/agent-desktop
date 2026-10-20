use std::time::Duration;

use agent_desktop_core::{AdapterError, Deadline, WindowInfo};

use super::focus_window::{FocusWorld, FrameView, Session};
use crate::system::bus_deadline::within;
use crate::system::display_server::{DisplayServer, session_server};
use crate::system::overview::overview_open;
use crate::system::proc_stat;
use crate::system::process_facts::{self, Lookup};
use crate::system::registry::toolkit;
use crate::system::restore_minimized::{self, Lifted};
use crate::system::toolkit::reports_global_position;
use crate::system::window_client::ClientWorld;
use crate::system::window_filter::frame_state;
use crate::system::x11::activate::{active_window_within, request_activation};
use crate::system::x11::activate_error::ActivateError;
use crate::system::x11::stacking_read::StackingRead;
use crate::system::x11::stacking_source::StackingSource;
use crate::system::x11::{DisplayReader, X11State};
use crate::system::{SharedBus, window_id};
use crate::tree::role_numbers::DIALOG;
use crate::tree::{AtspiElement, read_node};

#[path = "focus_send.rs"]
mod focus_send;

const ROOT_PATH: &str = "/org/a11y/atspi/accessible/root";
const ACTIVE_READ: Duration = Duration::from_millis(500);

/// The live desktop `focus_window` reads and writes: the accessibility bus
/// for the frame and the display server for the window manager.
pub(super) struct LiveWorld<'a> {
    bus: &'a SharedBus,
    frame: AtspiElement,
    deadline: Deadline,
}

impl<'a> LiveWorld<'a> {
    pub(super) fn new(
        bus: &'a SharedBus,
        win: &WindowInfo,
        deadline: Deadline,
    ) -> Result<Self, AdapterError> {
        let (bus_name, path) = window_id::decode(&win.id)?;
        Ok(Self {
            bus,
            frame: AtspiElement::new(bus_name, path, win.pid.get()),
            deadline,
        })
    }
}

impl FocusWorld for LiveWorld<'_> {
    fn session(&self) -> Session {
        match session_server() {
            DisplayServer::X11(_) => Session::X11,
            DisplayServer::Wayland => Session::Wayland,
            DisplayServer::Unavailable(..) => Session::NoDisplay,
        }
    }

    fn lookup(&self, pid: u32) -> Lookup {
        process_facts::lookup(pid)
    }

    fn frame(&self) -> Result<FrameView, AdapterError> {
        let shared = self.bus.get(self.deadline)?;
        let node = within(self.deadline, async {
            read_node(shared.inner(), &self.frame).await
        })?;
        let state = frame_state(&node.traits.states);
        Ok(FrameView {
            minimized: state.minimized,
            active: state.active,
            modal: state.modal,
            dialog: node.traits.role.present() == Some(&DIALOG),
        })
    }

    fn clients(&self) -> X11State {
        DisplayReader.clients_within(self.deadline)
    }

    fn stacking(&self, client: u32) -> StackingRead {
        DisplayReader.read_within(client, self.deadline)
    }

    fn send(&self, client: u32) -> Result<(), ActivateError> {
        request_activation(client)
    }

    fn active(&self) -> Option<u32> {
        active_window_within(ACTIVE_READ)
    }

    fn stopped(&self, pid: u32) -> bool {
        proc_stat::is_stopped(pid)
    }

    fn send_untimed(&self, client: u32) -> Result<(), ActivateError> {
        focus_send::request_untimed(client, self.deadline)
    }

    fn switch_desktop(&self, desktop: u32) -> Result<(), ActivateError> {
        focus_send::switch_desktop(desktop, self.deadline)
    }

    fn input_focus_in(&self, client: u32) -> bool {
        focus_send::input_focus_in(client, self.deadline)
    }

    fn overview_open(&self) -> Option<bool> {
        overview_open()
    }

    fn modal_over(&self, client: u32) -> Option<String> {
        focus_send::dialog_over(client, self.deadline)
    }

    fn lift(&self, client: u32) -> Result<Lifted, AdapterError> {
        restore_minimized::lift(client, self.deadline)
    }
}

impl ClientWorld for LiveWorld<'_> {
    fn global_position(&self) -> bool {
        let Ok(shared) = self.bus.get(self.deadline) else {
            return false;
        };
        let root = AtspiElement::new(self.frame.bus_name(), ROOT_PATH, self.frame.pid());
        let read = within(self.deadline, async {
            Ok(toolkit(shared.inner(), &root).await)
        });
        read.is_ok_and(|id| reports_global_position(id.name.as_deref(), id.version.as_deref()))
    }
}
