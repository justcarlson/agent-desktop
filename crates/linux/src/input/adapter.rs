use agent_desktop_core::{
    Action, AdapterError, ClipboardContent, ClipboardFormat, Deadline, DragParams, InputOps,
    InteractionLease, KeyCombo, MouseEvent, ObservationOps, OperationReport, RefEntry,
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
            || {
                let resolve = || self.resolve_element_strict(entry, deadline);
                bound_geometry::ref_receiver_refusal(resolve, self.bus(), deadline)
            },
        )
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
