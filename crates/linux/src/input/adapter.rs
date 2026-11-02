use agent_desktop_core::{
    Action, AdapterError, ClipboardContent, ClipboardFormat, Deadline, DragParams, InputOps,
    InteractionLease, KeyCombo, MouseEvent, OperationReport, RefEntry,
};

use crate::adapter::LinuxAdapter;

impl InputOps for LinuxAdapter {
    fn physical_pointer_refusal(&self) -> Option<AdapterError> {
        crate::input::hyprland::pointer_capability::session_pointer_refusal()
    }

    fn ref_pointer_refusal(
        &self,
        entry: &RefEntry,
        action: &Action,
        deadline: Deadline,
    ) -> Option<AdapterError> {
        use crate::input::hyprland::{bound_geometry, pointer_capability};
        pointer_capability::ref_gesture_refusal(
            action,
            || pointer_capability::session_hover_refusal(entry),
            || bound_geometry::ref_receiver_refusal(entry, deadline),
        )
    }

    /// A bound Hyprland activates through the compositor: the toolkit's own
    /// activation request after `GrabFocus` is refused on Hyprland 0.57.0
    /// (L48-1). Other Wayland sessions have no activation route.
    fn keyboard_needs_window_activation(&self) -> bool {
        match crate::input::backend::backend() {
            Ok(crate::input::backend::Backend::Wayland) => {
                !matches!(crate::input::hyprland::hypr_route::bound_route(), Ok(None))
            }
            _ => true,
        }
    }

    fn mouse_event(
        &self,
        event: MouseEvent,
        lease: &InteractionLease,
    ) -> Result<OperationReport, AdapterError> {
        crate::input::deliver::mouse_event(&event, lease)
    }

    fn drag(
        &self,
        params: DragParams,
        lease: &InteractionLease,
    ) -> Result<OperationReport, AdapterError> {
        crate::input::deliver::drag(&params, lease)
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
