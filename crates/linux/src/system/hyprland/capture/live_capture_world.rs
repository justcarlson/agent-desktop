//! The live compositor behind Hyprland capture. Every connection is to the
//! bound compositor's own sockets, checked by peer, and every read and
//! frame carries the command deadline.

use agent_desktop_core::{AdapterError, Deadline, DisplayInfo, WindowInfo};

use super::capture_error::capture_error;
use super::capture_world::CaptureWorld;
use super::enforce_option::read_enforced;
use super::toplevel_composition::composition_of;
use crate::system::hyprland::clients::client_read::{read_clients, read_desktop, read_filling};
use crate::system::hyprland::hyprland_binding::HyprlandBinding;
use crate::system::hyprland::identity::bound_toplevel::BoundToplevel;
use crate::system::hyprland::identity::inventory::Inventory;
use crate::system::hyprland::identity::window_bind::bind_window;
use crate::system::hyprland::monitors;
use crate::system::process::process_facts;
use crate::system::process::process_lookup::Lookup;
use crate::system::wayland::bound_connection::connect_bound;
use crate::system::wayland::capture::capture_fault::CaptureFault;
use crate::system::wayland::capture::composition::Composition;
use crate::system::wayland::capture::frame_failure::FrameFailure;
use crate::system::wayland::capture::output_frame::capture_output;
use crate::system::wayland::capture::raw_frame::RawFrame;
use crate::system::wayland::capture::toplevel_frame::capture_toplevel;
use crate::system::wayland::toplevel::toplevel_session::ToplevelSession;

/// One bound compositor and, once a window is bound, its foreign toplevel
/// session.
pub(crate) struct LiveCaptureWorld {
    binding: HyprlandBinding,
    deadline: Deadline,
    session: Option<ToplevelSession>,
}

impl LiveCaptureWorld {
    pub(crate) fn new(binding: HyprlandBinding, deadline: Deadline) -> Self {
        Self {
            binding,
            deadline,
            session: None,
        }
    }

    fn socket(&self) -> String {
        self.binding.wayland_socket.display().to_string()
    }

    fn session(&mut self) -> Result<&mut ToplevelSession, AdapterError> {
        match &mut self.session {
            Some(session) => Ok(session),
            empty => Ok(empty.insert(ToplevelSession::open(&self.binding, self.deadline)?)),
        }
    }
}

impl CaptureWorld for LiveCaptureWorld {
    fn enforced(&mut self) -> Result<bool, AdapterError> {
        read_enforced(&self.binding, self.deadline)
    }

    fn bind(&mut self, win: &WindowInfo) -> Result<BoundToplevel, AdapterError> {
        let clients = read_clients(&self.binding, self.deadline)?;
        let toplevels = self.session()?.views()?;
        let live = match process_facts::lookup(win.pid.get()) {
            Lookup::Found(facts) => Some(facts.instance),
            Lookup::Gone | Lookup::Unreadable => None,
        };
        let inventory = Inventory {
            clients: &clients,
            toplevels: &toplevels,
        };
        let compositor = (self.binding.pid, self.binding.start_time);
        bind_window(win, inventory, live.as_deref(), compositor)
    }

    fn composition(&mut self, bound: &BoundToplevel) -> Result<Composition, AdapterError> {
        let desktop = read_desktop(&self.binding, self.deadline)?;
        let filling = read_filling(&self.binding, self.deadline)?;
        Ok(composition_of(&bound.client, &desktop, filling.as_deref()))
    }

    fn capture_toplevel(
        &mut self,
        identifier: &str,
        composition: Composition,
    ) -> Result<RawFrame, AdapterError> {
        let socket = self.socket();
        let deadline = self.deadline;
        let session = self.session()?;
        let handle = session
            .ext_handle(identifier)
            .ok_or_else(|| capture_error(&CaptureFault::Failed(FrameFailure::Stopped), &socket))?;
        capture_toplevel(session.connection(), &handle, composition, deadline)
            .map_err(|fault| capture_error(&fault, &socket))
    }

    fn still_bound(&mut self, bound: &BoundToplevel) -> Result<bool, AdapterError> {
        let deadline = self.deadline;
        let session = self.session()?;
        session.settle(deadline)?;
        let identifier = &bound.toplevel.identifier;
        let mapped = session
            .views()?
            .iter()
            .any(|view| view.identifier == *identifier && view.address == bound.client.address);
        Ok(mapped && session.ext_handle(identifier).is_some())
    }

    fn displays(&mut self) -> Result<Vec<DisplayInfo>, AdapterError> {
        monitors::list(&self.binding, self.deadline)
    }

    fn capture_output(&mut self, name: &str) -> Result<RawFrame, AdapterError> {
        let connection = connect_bound(&self.binding)?;
        capture_output(&connection, name, self.deadline)
            .map_err(|fault| capture_error(&fault, &self.socket()))
    }
}
