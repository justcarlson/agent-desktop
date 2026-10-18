use agent_desktop_core::{
    AccessibilityNode, ActionOps, AdapterError, AppInfo, AppInventory, Deadline, InputOps,
    ObservationOps, ObservationRequest, ObservationRoot, ObservedTree, TreeOptions, WindowFilter,
    WindowInfo,
};

use crate::system::SharedBus;

pub struct LinuxAdapter {
    bus: SharedBus,
}

impl LinuxAdapter {
    pub fn new() -> Self {
        Self {
            bus: SharedBus::default(),
        }
    }

    pub(crate) fn bus(&self) -> &SharedBus {
        &self.bus
    }
}

impl Default for LinuxAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl ObservationOps for LinuxAdapter {
    fn observe_tree(
        &self,
        root: ObservationRoot<'_>,
        request: &ObservationRequest,
    ) -> Result<ObservedTree, AdapterError> {
        crate::tree::observe_tree(self.bus(), root, request)
    }

    fn list_windows(
        &self,
        filter: &WindowFilter,
        deadline: Deadline,
    ) -> Result<Vec<WindowInfo>, AdapterError> {
        crate::system::list_windows(self.bus(), filter, deadline)
    }

    fn list_apps(&self, deadline: Deadline) -> Result<Vec<AppInfo>, AdapterError> {
        Ok(self.list_apps_inventory(deadline)?.apps)
    }

    fn list_apps_inventory(&self, deadline: Deadline) -> Result<AppInventory, AdapterError> {
        crate::system::app_inventory(self.bus(), deadline)
    }

    fn list_apps_scoped(
        &self,
        name: &str,
        bundle_id: Option<&str>,
        deadline: Deadline,
    ) -> Result<Vec<AppInfo>, AdapterError> {
        crate::system::apps_scoped(self.bus(), name, bundle_id, deadline)
    }

    fn get_tree(
        &self,
        window: &WindowInfo,
        options: &TreeOptions,
        deadline: Deadline,
    ) -> Result<AccessibilityNode, AdapterError> {
        self.observe_tree(
            ObservationRoot::Window(window),
            &ObservationRequest::snapshot(options, deadline),
        )?
        .into_accessibility_tree()
    }
}
impl ActionOps for LinuxAdapter {}
impl InputOps for LinuxAdapter {}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_desktop_core::{AppError, CommandContext, ErrorCode, SnapshotSurface, SystemOps};

    #[test]
    fn snapshot_surfaces_fail_closed_until_linux_implements_them() {
        let adapter = LinuxAdapter::new();
        assert!(adapter.supported_surfaces().is_empty());

        let error = agent_desktop_core::commands::snapshot::execute(
            agent_desktop_core::commands::snapshot::SnapshotArgs {
                app: None,
                window_id: None,
                max_depth: 1,
                include_bounds: false,
                interactive_only: false,
                compact: true,
                surface: SnapshotSurface::Window,
                skeleton: false,
                root_ref: None,
                timeout_ms: None,
                force_electron_a11y: false,
            },
            &adapter,
            &CommandContext::default(),
        )
        .expect_err("an unimplemented surface must fail at validation");

        let AppError::Adapter(error) = error else {
            panic!("surface validation must return an adapter error")
        };
        assert_eq!(error.code, ErrorCode::PlatformNotSupported);
        assert!(
            error
                .details
                .as_ref()
                .and_then(|details| details.get("supported_surfaces"))
                .and_then(|surfaces| surfaces.as_array())
                .is_some_and(Vec::is_empty)
        );
    }
}
