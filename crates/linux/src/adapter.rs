use agent_desktop_core::{
    AccessibilityNode, ActionOps, ActionRequest, ActionResult, AdapterError, AppInfo, AppInventory,
    ClipboardContent, ClipboardFormat, Deadline, DragParams, ElementState, HitTestResult, InputOps,
    InteractionLease, KeyCombo, LiveElement, MouseEvent, NativeHandle, ObservationOps,
    ObservationRequest, ObservationRoot, ObservedTree, OperationReport, Point, ProcessIdentity,
    Rect, RefEntry, SnapshotSurface, SurfaceInfo, TreeOptions, WindowFilter, WindowInfo,
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
        if let ObservationRoot::Element { handle, .. } = &root
            && let Some(target) = crate::tray::target_of(handle)
        {
            return crate::tray::observe::observe(target, root, request.deadline);
        }
        crate::tree::observe_tree(self.bus(), root, request)
    }

    fn resolve_element_strict(
        &self,
        entry: &RefEntry,
        deadline: Deadline,
    ) -> Result<NativeHandle, AdapterError> {
        if crate::tray::owns(entry) {
            return crate::tray::resolve(entry, deadline);
        }
        crate::tree::resolve_element_strict(self.bus(), entry, deadline)
    }

    fn resolve_locator_anchor(
        &self,
        entry: &RefEntry,
        deadline: Deadline,
    ) -> Result<NativeHandle, AdapterError> {
        if crate::tray::owns(entry) {
            return crate::tray::resolve(entry, deadline);
        }
        if crate::tree::is_menu_entry(entry) {
            return crate::tree::resolve_element_strict(self.bus(), entry, deadline);
        }
        crate::tree::resolve_locator_anchor(self.bus(), entry, deadline)
    }

    fn resolve_shell_surface(
        &self,
        surface: SnapshotSurface,
        deadline: Deadline,
    ) -> Result<WindowInfo, AdapterError> {
        crate::system::shell::resolve_surface(self.bus(), surface, deadline)
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
        if let Some(target) = crate::tray::target_of(handle) {
            return crate::tray::live_state(target, deadline);
        }
        crate::tree::get_live_state(self.bus(), handle, deadline)
    }

    fn get_live_actions(
        &self,
        handle: &NativeHandle,
        deadline: Deadline,
    ) -> Result<Option<Vec<String>>, AdapterError> {
        if let Some(target) = crate::tray::target_of(handle) {
            return crate::tray::live_actions(target, deadline);
        }
        crate::tree::get_live_actions(self.bus(), handle, deadline)
    }

    fn get_live_element(
        &self,
        handle: &NativeHandle,
        deadline: Deadline,
    ) -> Result<LiveElement, AdapterError> {
        if let Some(target) = crate::tray::target_of(handle) {
            return crate::tray::live_element(target, deadline);
        }
        let element = crate::tree::get_live_element(self.bus(), handle, deadline)?;
        if element.bounds.is_none() && crate::tree::inside_popup(self.bus(), handle, deadline) {
            return Err(crate::tree::no_position());
        }
        Ok(element)
    }

    fn get_element_bounds(
        &self,
        handle: &NativeHandle,
        deadline: Deadline,
    ) -> Result<Option<Rect>, AdapterError> {
        if let Some(target) = crate::tray::target_of(handle) {
            return crate::tray::bounds(target, deadline);
        }
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
        if let Some(target) = crate::tray::target_of(handle) {
            return crate::tray::hit_test(target, point, deadline);
        }
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

impl LinuxAdapter {
    fn deliver(
        &self,
        handle: &NativeHandle,
        request: ActionRequest,
        lease: &InteractionLease,
    ) -> Result<ActionResult, AdapterError> {
        let target = crate::tree::atspi_element(handle)
            .ok()
            .map(|element| element.pid());
        let result = match crate::tray::target_of(handle) {
            Some(target) => crate::tray::execute(target, request, lease),
            None => {
                let outcome = crate::tree::atspi_element(handle)
                    .ok()
                    .filter(|element| crate::system::shell::is_shell_element(element))
                    .and_then(|element| {
                        crate::system::shell::deliver::deliver(self.bus(), element, &request, lease)
                    });
                match outcome {
                    Some(result) => result,
                    None => crate::actions::execute_action(self.bus(), handle, request, lease),
                }
            }
        };
        crate::system::action_clock::mark_target(target);
        result
    }
}

impl ActionOps for LinuxAdapter {
    fn execute_action(
        &self,
        handle: &NativeHandle,
        request: ActionRequest,
        lease: &InteractionLease,
    ) -> Result<ActionResult, AdapterError> {
        self.deliver(handle, request, lease)
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
    fn physical_pointer_refusal(&self) -> Option<AdapterError> {
        match crate::input::backend::backend() {
            Ok(crate::input::backend::Backend::Wayland) => Some(
                crate::input::wayland::keysym_send::pointer_refusal("physical pointer input"),
            ),
            _ => None,
        }
    }

    fn keyboard_needs_window_activation(&self) -> bool {
        !matches!(
            crate::input::backend::backend(),
            Ok(crate::input::backend::Backend::Wayland)
        )
    }

    fn mouse_event(
        &self,
        event: MouseEvent,
        lease: &InteractionLease,
    ) -> Result<OperationReport, AdapterError> {
        crate::input::deliver::mouse_event(&event, lease.deadline())
    }

    fn drag(
        &self,
        params: DragParams,
        lease: &InteractionLease,
    ) -> Result<OperationReport, AdapterError> {
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

    fn get_clipboard_content(
        &self,
        format: ClipboardFormat,
        deadline: Deadline,
    ) -> Result<Option<ClipboardContent>, AdapterError> {
        crate::input::clipboard::set_clear::get_clipboard(format, deadline)
    }

    fn set_clipboard_content(
        &self,
        content: &ClipboardContent,
        lease: &InteractionLease,
    ) -> Result<OperationReport, AdapterError> {
        crate::input::clipboard::set_clear::set_clipboard(content, lease)?;
        Ok(OperationReport::clipboard_written("clipboard-set", false))
    }

    fn clear_clipboard(&self, lease: &InteractionLease) -> Result<OperationReport, AdapterError> {
        crate::input::clipboard::set_clear::clear_clipboard(lease)?;
        Ok(OperationReport::clipboard_written("clipboard-clear", false))
    }
}

#[cfg(test)]
#[path = "adapter_tests.rs"]
mod tests;
