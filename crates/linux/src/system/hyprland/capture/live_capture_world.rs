//! The live compositor behind Hyprland capture. Every connection is to the
//! bound compositor's own sockets, checked by peer, and every read and
//! frame carries the command deadline.

use agent_desktop_core::{AdapterError, Deadline, DisplayInfo};

use super::capture_error::capture_error;
use super::capture_world::CaptureWorld;
use super::enforce_option::read_enforced;
use crate::system::display::display_wayland::list_outputs;
use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;
use crate::system::hyprland::toplevel::bound_socket::connect_bound;
use crate::system::wayland::capture::output_frame::capture_output;
use crate::system::wayland::capture::raw_frame::RawFrame;

/// One bound compositor and the deadline every read shares.
pub(crate) struct LiveCaptureWorld {
    binding: HyprlandBinding,
    deadline: Deadline,
}

impl LiveCaptureWorld {
    pub(crate) fn new(binding: HyprlandBinding, deadline: Deadline) -> Self {
        Self { binding, deadline }
    }
}

impl CaptureWorld for LiveCaptureWorld {
    fn enforced(&mut self) -> Result<bool, AdapterError> {
        read_enforced(&self.binding, self.deadline)
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
