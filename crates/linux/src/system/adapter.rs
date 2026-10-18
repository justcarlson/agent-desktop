use agent_desktop_core::{
    AdapterError, AdapterSession, Deadline, InteractionLease, PermissionReport, SessionAffinity,
    SystemOps,
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

    fn open_session(
        &self,
        _affinity: &SessionAffinity,
        deadline: Deadline,
    ) -> Result<Box<dyn AdapterSession>, AdapterError> {
        Ok(Box::new(session::open(self.bus(), deadline)?))
    }
}
