//! One held toplevel session to the bound compositor that reads a process's
//! toplevels and sends each a close request.

use agent_desktop_core::{AdapterError, Deadline, ErrorCode};

use super::owned_toplevels::{check_owner, owned_toplevels};
use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;
use crate::system::hyprland::clients::client_read::read_clients;
use crate::system::hyprland::clients::hypr_client::HyprClient;
use crate::system::hyprland::identity::inventory_read::stable_inventory;
use crate::system::hyprland::toplevel::toplevel_session::ToplevelSession;
use crate::system::hyprland::toplevel::wlr_window::WlrWindow;
use crate::system::process::identity::Identity;
use crate::system::process::liveness;
use crate::system::x11::control::ControlRead;
use crate::system::x11::control::control_error::ControlError;
use crate::system::x11::window::toplevel::Toplevel;

/// The close route to one process's toplevels on the bound compositor.
pub(crate) struct CompositorClose {
    binding: HyprlandBinding,
    session: ToplevelSession,
    deadline: Deadline,
    pid: u32,
}

impl CompositorClose {
    /// Opens the toplevel session. A compositor that does not advertise
    /// `zwlr_foreign_toplevel_manager_v1` or the mapping manager is
    /// `PLATFORM_NOT_SUPPORTED`.
    pub(crate) fn open(
        binding: HyprlandBinding,
        pid: u32,
        deadline: Deadline,
    ) -> Result<Self, AdapterError> {
        let session = ToplevelSession::open(&binding, deadline)?;
        Ok(Self {
            binding,
            session,
            deadline,
            pid,
        })
    }

    /// Every toplevel of the process as the compositor shows it now.
    pub(crate) fn toplevels(&mut self) -> ControlRead<Vec<Toplevel>> {
        match self.read() {
            Ok((windows, clients)) => {
                ControlRead::Ready(owned_toplevels(&windows, &clients, self.pid))
            }
            Err(_) => ControlRead::Unavailable,
        }
    }

    /// The process's toplevels over a fresh session that waits until
    /// `deadline`, for a read after the session's own deadline has passed.
    pub(crate) fn toplevels_until(&self, deadline: Deadline) -> ControlRead<Vec<Toplevel>> {
        match Self::open(self.binding.clone(), self.pid, deadline) {
            Ok(mut fresh) => fresh.toplevels(),
            Err(_) => ControlRead::Unavailable,
        }
    }

    /// Sends a close request to toplevel `id` once the process `instance`
    /// names still runs and still owns it.
    pub(crate) fn request_close(&mut self, id: u32, instance: &str) -> Result<(), ControlError> {
        if self.deadline.is_expired() {
            return Err(ControlError::TimedOut);
        }
        if liveness::identity(self.pid, instance) != Identity::Same {
            return Err(ControlError::StaleWindow);
        }
        let (windows, clients) = self.read().map_err(|error| self.read_failure(&error))?;
        check_owner(id, &windows, &clients, self.pid)?;
        match self.session.close(id) {
            Ok(true) => Ok(()),
            Ok(false) => Err(ControlError::BadWindow),
            Err(_) => Err(ControlError::SendFailed),
        }
    }

    /// A revalidation read that ran out of the deadline is a timeout; any
    /// other failed read means the compositor could not be reached.
    fn read_failure(&self, error: &AdapterError) -> ControlError {
        if error.code == ErrorCode::Timeout || self.deadline.is_expired() {
            ControlError::TimedOut
        } else {
            ControlError::NoDisplay
        }
    }

    /// The toplevels and the clients that one read agrees on, so a closed
    /// window's reused address never hands its handle to a new owner.
    fn read(&mut self) -> Result<(Vec<WlrWindow>, Vec<HyprClient>), AdapterError> {
        let (binding, deadline, session) = (&self.binding, self.deadline, &mut self.session);
        let (clients, windows) = stable_inventory(
            || read_clients(binding, deadline),
            || {
                session.settle()?;
                Ok(session.windows())
            },
        )?;
        Ok((windows, clients))
    }
}

#[cfg(test)]
#[path = "compositor_close_tests.rs"]
mod tests;
