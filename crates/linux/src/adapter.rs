use agent_desktop_core::{
    AccessibilityNode, ActionOps, ActionRequest, ActionResult, AdapterError, AppInfo, AppInventory,
    Deadline, DragParams, ElementState, HitTestResult, InputOps, InteractionLease, KeyCombo,
    LiveElement, MouseEvent, NativeHandle, ObservationOps, ObservationRequest, ObservationRoot,
    ObservedTree, Point, ProcessIdentity, Rect, RefEntry, SurfaceInfo, TreeOptions, WindowFilter,
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

    fn resolve_element_strict(
        &self,
        entry: &RefEntry,
        deadline: Deadline,
    ) -> Result<NativeHandle, AdapterError> {
        crate::tree::resolve_element_strict(self.bus(), entry, deadline)
    }

    fn resolve_locator_anchor(
        &self,
        entry: &RefEntry,
        deadline: Deadline,
    ) -> Result<NativeHandle, AdapterError> {
        crate::tree::resolve_locator_anchor(self.bus(), entry, deadline)
    }

    fn get_live_value(
        &self,
        handle: &NativeHandle,
        deadline: Deadline,
    ) -> Result<Option<String>, AdapterError> {
        crate::tree::get_live_value(self.bus(), handle, deadline)
    }

    fn get_live_state(
        &self,
        handle: &NativeHandle,
        deadline: Deadline,
    ) -> Result<Option<ElementState>, AdapterError> {
        crate::tree::get_live_state(self.bus(), handle, deadline)
    }

    fn get_live_actions(
        &self,
        handle: &NativeHandle,
        deadline: Deadline,
    ) -> Result<Option<Vec<String>>, AdapterError> {
        crate::tree::get_live_actions(self.bus(), handle, deadline)
    }

    fn get_live_element(
        &self,
        handle: &NativeHandle,
        deadline: Deadline,
    ) -> Result<LiveElement, AdapterError> {
        crate::tree::get_live_element(self.bus(), handle, deadline)
    }

    fn get_element_bounds(
        &self,
        handle: &NativeHandle,
        deadline: Deadline,
    ) -> Result<Option<Rect>, AdapterError> {
        crate::tree::get_element_bounds(self.bus(), handle, deadline)
    }

    fn get_text_selection(
        &self,
        handle: &NativeHandle,
        deadline: Deadline,
    ) -> Result<Option<std::ops::Range<usize>>, AdapterError> {
        crate::tree::text_selection::read_selection(self.bus(), handle, deadline)
    }

    fn hit_test(
        &self,
        handle: &NativeHandle,
        point: Point,
        deadline: Deadline,
    ) -> Result<HitTestResult, AdapterError> {
        crate::tree::hit_test(self.bus(), handle, point, deadline)
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

    fn list_surfaces(
        &self,
        process: ProcessIdentity,
        deadline: Deadline,
    ) -> Result<Vec<SurfaceInfo>, AdapterError> {
        crate::system::list_surfaces(self.bus(), &process, deadline)
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

impl ActionOps for LinuxAdapter {
    fn execute_action(
        &self,
        handle: &NativeHandle,
        request: ActionRequest,
        lease: &InteractionLease,
    ) -> Result<ActionResult, AdapterError> {
        crate::actions::execute_action(self.bus(), handle, request, lease)
    }

    fn scroll_into_view(
        &self,
        handle: &NativeHandle,
        lease: &InteractionLease,
    ) -> Result<(), AdapterError> {
        crate::actions::scroll_into_view(self.bus(), handle, lease)
    }
}
impl InputOps for LinuxAdapter {
    fn mouse_event(&self, event: MouseEvent, lease: &InteractionLease) -> Result<(), AdapterError> {
        crate::input::deliver::mouse_event(&event, lease.deadline())
    }

    fn drag(&self, params: DragParams, lease: &InteractionLease) -> Result<(), AdapterError> {
        crate::input::deliver::drag(&params, lease.deadline())
    }

    fn key_event(
        &self,
        _combo: &KeyCombo,
        _down: bool,
        _lease: &InteractionLease,
    ) -> Result<(), AdapterError> {
        Err(crate::input::standalone::reject_standalone(
            "key-down/key-up",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_desktop_core::{AppError, CommandContext, ErrorCode, SnapshotSurface, SystemOps};

    fn snapshot_args(
        surface: SnapshotSurface,
    ) -> agent_desktop_core::commands::snapshot::SnapshotArgs {
        agent_desktop_core::commands::snapshot::SnapshotArgs {
            app: None,
            window_id: None,
            max_depth: 1,
            include_bounds: false,
            interactive_only: false,
            compact: true,
            surface,
            skeleton: false,
            root_ref: None,
            timeout_ms: None,
            force_electron_a11y: false,
        }
    }

    #[test]
    fn supported_surfaces_advertises_window_and_sheet_and_rejects_the_rest() {
        let adapter = LinuxAdapter::new();
        assert_eq!(
            adapter.supported_surfaces(),
            vec![SnapshotSurface::Window, SnapshotSurface::Sheet]
        );

        let error = agent_desktop_core::commands::snapshot::execute(
            snapshot_args(SnapshotSurface::Menu),
            &adapter,
            &CommandContext::default(),
        )
        .expect_err("an unsupported surface must fail at validation");

        let AppError::Adapter(error) = error else {
            panic!("surface validation must return an adapter error")
        };
        assert_eq!(error.code, ErrorCode::PlatformNotSupported);
        assert_eq!(
            error
                .details
                .as_ref()
                .and_then(|details| details.get("supported_surfaces"))
                .and_then(|surfaces| surfaces.as_array())
                .map(Vec::len),
            Some(2)
        );
    }

    #[test]
    fn a_window_surface_passes_validation_and_reaches_the_adapter() {
        let adapter = LinuxAdapter::new();
        let error = agent_desktop_core::commands::snapshot::execute(
            snapshot_args(SnapshotSurface::Window),
            &adapter,
            &CommandContext::default(),
        )
        .expect_err("no session can answer in a unit test");

        let AppError::Adapter(error) = error else {
            panic!("the observation must fail as an adapter error")
        };
        assert!(
            error
                .details
                .as_ref()
                .and_then(|details| details.get("requested_surface"))
                .is_none(),
            "the failure must come from the adapter, not surface validation: {error:?}"
        );
    }

    #[test]
    fn every_live_read_is_the_adapters_own_and_not_the_unsupported_default() {
        let adapter = LinuxAdapter::new();
        let foreign = agent_desktop_core::NativeHandle::new(5u32);
        let deadline = agent_desktop_core::Deadline::after(1_000).expect("a deadline builds");
        let errors = [
            adapter.get_live_value(&foreign, deadline).err(),
            adapter.get_live_state(&foreign, deadline).err(),
            adapter.get_live_actions(&foreign, deadline).err(),
            adapter.get_live_element(&foreign, deadline).err(),
            adapter.get_element_bounds(&foreign, deadline).err(),
        ];
        for error in errors {
            let error = error.expect("a foreign handle is refused");
            assert_eq!(error.code, ErrorCode::InvalidArgs, "{error:?}");
        }
    }
}
