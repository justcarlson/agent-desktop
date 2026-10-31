//! The live compositor behind Hyprland capture. Every connection is to the
//! bound compositor's own sockets, checked by peer, and every read and
//! frame carries the command deadline. A window capture holds the toplevel
//! reader that bound it, because the capture passes that connection's
//! handle to the compositor.

use agent_desktop_core::{AdapterError, Deadline, DisplayInfo, WindowInfo};

use super::capture_error::capture_error;
use super::capture_world::CaptureWorld;
use super::enforce_option::read_enforced;
use super::toplevel_composition::composition_of;
use crate::system::display::display_wayland::list_outputs;
use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;
use crate::system::hyprland::clients::client_read::{read_clients, read_desktop, read_filling};
use crate::system::hyprland::identity::bound_toplevel::BoundToplevel;
use crate::system::hyprland::identity::inventory::Inventory;
use crate::system::hyprland::identity::window_bind::bind_window;
use crate::system::hyprland::toplevel::bound_socket::connect_bound;
use crate::system::hyprland::toplevel::toplevel_read::read_toplevels;
use crate::system::hyprland::toplevel::toplevel_reader::ToplevelReader;
use crate::system::process::process_facts;
use crate::system::process::process_lookup::Lookup;
use crate::system::wayland::capture::capture_fault::CaptureFault;
use crate::system::wayland::capture::composition::Composition;
use crate::system::wayland::capture::frame_failure::FrameFailure;
use crate::system::wayland::capture::output_frame::capture_output;
use crate::system::wayland::capture::raw_frame::RawFrame;
use crate::system::wayland::capture::toplevel_frame::capture_toplevel;

/// One bound compositor, the deadline every read shares and, once a window
/// is bound, the reader that holds its toplevel handle.
pub(crate) struct LiveCaptureWorld {
    binding: HyprlandBinding,
    deadline: Deadline,
    reader: Option<ToplevelReader>,
}

impl LiveCaptureWorld {
    pub(crate) fn new(binding: HyprlandBinding, deadline: Deadline) -> Self {
        Self {
            binding,
            deadline,
            reader: None,
        }
    }

    fn reader(&mut self) -> Result<&ToplevelReader, AdapterError> {
        match &mut self.reader {
            Some(reader) => Ok(reader),
            empty => Ok(empty.insert(ToplevelReader::open(&self.binding, self.deadline)?)),
        }
    }
}

impl CaptureWorld for LiveCaptureWorld {
    fn enforced(&mut self) -> Result<bool, AdapterError> {
        read_enforced(&self.binding, self.deadline)
    }

    fn bind(&mut self, win: &WindowInfo) -> Result<BoundToplevel, AdapterError> {
        let clients = read_clients(&self.binding, self.deadline)?;
        let toplevels = self.reader()?.views()?;
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
        let deadline = self.deadline;
        let socket = self.binding.wayland_socket.clone();
        let reader = self.reader()?;
        let handle = reader
            .ext_handle(identifier)
            .ok_or_else(|| capture_error(&CaptureFault::Failed(FrameFailure::Stopped), &socket))?;
        capture_toplevel(reader.connection(), &handle, composition, deadline)
            .map_err(|fault| capture_error(&fault, &socket))
    }

    fn still_bound(&mut self, bound: &BoundToplevel) -> Result<bool, AdapterError> {
        let identifier = &bound.toplevel.identifier;
        Ok(read_toplevels(&self.binding, self.deadline)?
            .iter()
            .any(|view| view.identifier == *identifier && view.address == bound.client.address))
    }

    fn displays(&mut self) -> Result<Vec<DisplayInfo>, AdapterError> {
        list_outputs(&self.binding.wayland_socket, self.deadline)
    }

    fn capture_output(&mut self, name: &str) -> Result<RawFrame, AdapterError> {
        let stream = connect_bound(&self.binding)?;
        capture_output(stream, name, self.deadline)
            .map_err(|fault| capture_error(&fault, &self.binding.wayland_socket))
    }
}
