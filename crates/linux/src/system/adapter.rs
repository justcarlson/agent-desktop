use agent_desktop_core::{
    AdapterError, AdapterSession, Deadline, InteractionLease, PermissionReport, SessionAffinity,
    SystemOps, WindowInfo,
};

use crate::adapter::LinuxAdapter;
use crate::system::{interaction_lease, permissions, session};

impl SystemOps for LinuxAdapter {
    fn acquire_interaction_lease(
        &self,
        deadline: Deadline,
    ) -> Result<InteractionLease, AdapterError> {
        interaction_lease::acquire(deadline)
    }

    fn permission_report(&self, deadline: Deadline) -> Result<PermissionReport, AdapterError> {
        permissions::report(self.bus(), deadline)
    }

    fn focused_window(&self, deadline: Deadline) -> Result<Option<WindowInfo>, AdapterError> {
        crate::system::focused_window(self.bus(), deadline)
    }

    fn resolve_window_strict(
        &self,
        win: &WindowInfo,
        deadline: Deadline,
    ) -> Result<WindowInfo, AdapterError> {
        crate::system::resolve_window_strict(self.bus(), win, deadline)
    }

    fn open_session(
        &self,
        _affinity: &SessionAffinity,
        deadline: Deadline,
    ) -> Result<Box<dyn AdapterSession>, AdapterError> {
        Ok(Box::new(session::open(self.bus(), deadline)?))
    }
}
