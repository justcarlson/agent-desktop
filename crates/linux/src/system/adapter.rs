use agent_desktop_core::{
    AdapterError, AdapterSession, Deadline, PermissionReport, SessionAffinity, SystemOps,
};

use crate::adapter::LinuxAdapter;
use crate::system::{permissions, session};

impl SystemOps for LinuxAdapter {
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
