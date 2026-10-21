use agent_desktop_core::launch_options::LaunchOptions;
use agent_desktop_core::launch_result::LaunchResult;
use agent_desktop_core::process_state::ProcessState;
use agent_desktop_core::{
    ActionResult, AdapterError, AdapterSession, AppInfo, Deadline, DismissAllNotificationsRequest,
    DismissNotificationRequest, DisplayInfo, ImageBuffer, InteractionLease, InteractionPolicy,
    KeyCombo, NotificationActionRequest, NotificationFilter, NotificationInfo, OperationReport,
    PermissionReport, ProcessIdentity, ScreenshotTarget, SessionAffinity, SignalBaseline,
    SignalFilter, SnapshotSurface, SystemOps, WindowInfo, WindowOp,
};

use crate::adapter::LinuxAdapter;
use crate::notifications;
use crate::system::{
    action_clock, close, interaction_lease, key_dispatch, launch, menu_surface, menu_wait,
    permissions, process_state, protected, session, shell, signal_capture, window_op,
};

impl SystemOps for LinuxAdapter {
    fn update_cursor_overlay(
        &self,
        control: &agent_desktop_core::CursorOverlayControl,
    ) -> Result<(), AdapterError> {
        super::cursor_overlay::update(control)
    }

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

    fn list_displays(&self, deadline: Deadline) -> Result<Vec<DisplayInfo>, AdapterError> {
        crate::system::list_displays(deadline)
    }

    fn focus_window(&self, win: &WindowInfo, lease: &InteractionLease) -> Result<(), AdapterError> {
        crate::system::focus_window::focus_window(self.bus(), win, lease)
    }

    fn close_app(
        &self,
        app: &AppInfo,
        force: bool,
        lease: &InteractionLease,
    ) -> Result<OperationReport, AdapterError> {
        close::close_app(self.bus(), app, force, lease)
    }

    fn is_protected_process(&self, identifier: &str) -> bool {
        protected::is_protected(identifier)
    }

    fn launch_app(
        &self,
        id: &str,
        options: &LaunchOptions,
        lease: &InteractionLease,
    ) -> Result<LaunchResult, AdapterError> {
        launch::launch_app(self.bus(), id, options, lease)
    }

    fn process_state(
        &self,
        process: ProcessIdentity,
        deadline: Deadline,
    ) -> Result<ProcessState, AdapterError> {
        process_state::process_state(self.bus(), process, deadline)
    }

    fn window_op(
        &self,
        win: &WindowInfo,
        op: WindowOp,
        lease: &InteractionLease,
    ) -> Result<OperationReport, AdapterError> {
        window_op::window_op(self.bus(), win, op, lease)
    }

    fn press_key_for_app(
        &self,
        process: ProcessIdentity,
        combo: &KeyCombo,
        policy: InteractionPolicy,
        lease: &InteractionLease,
    ) -> Result<ActionResult, AdapterError> {
        let pid = process.pid.get();
        let result = key_dispatch::press_key_for_app(self.bus(), process, combo, policy, lease);
        action_clock::mark_process(pid);
        result
    }

    fn press_key_for_window(
        &self,
        window: &WindowInfo,
        combo: &KeyCombo,
        policy: InteractionPolicy,
        lease: &InteractionLease,
    ) -> Result<ActionResult, AdapterError> {
        let result = key_dispatch::press_key_for_window(self.bus(), window, combo, policy, lease);
        action_clock::mark_process(window.pid.get());
        result
    }

    fn is_blocked_combo(&self, combo: &KeyCombo) -> bool {
        crate::input::blocked_combo::is_blocked(combo)
    }

    fn screenshot(
        &self,
        target: ScreenshotTarget,
        deadline: Deadline,
    ) -> Result<ImageBuffer, AdapterError> {
        crate::system::screenshot::screenshot(self.bus(), target, deadline)
    }

    fn screenshot_window_frame(
        &self,
        window: &WindowInfo,
        deadline: Deadline,
    ) -> Result<ImageBuffer, AdapterError> {
        crate::system::screenshot::screenshot_window_frame(self.bus(), window, deadline)
    }

    fn capture_signal_baseline(
        &self,
        filter: &SignalFilter,
        deadline: Deadline,
    ) -> Result<SignalBaseline, AdapterError> {
        signal_capture::capture_signal_baseline(self.bus(), filter, deadline)
    }

    fn wait_for_menu(
        &self,
        process: ProcessIdentity,
        open: bool,
        deadline: Deadline,
    ) -> Result<(), AdapterError> {
        menu_wait::wait_for_menu(self.bus(), process, open, deadline)
    }

    fn supported_surfaces(&self) -> Vec<SnapshotSurface> {
        let mut surfaces = vec![SnapshotSurface::Window, SnapshotSurface::Sheet];
        surfaces.extend(menu_surface::supported());
        surfaces.extend(shell::supported(self.bus()));
        surfaces
    }

    fn open_session(
        &self,
        _affinity: &SessionAffinity,
        deadline: Deadline,
    ) -> Result<Box<dyn AdapterSession>, AdapterError> {
        Ok(Box::new(session::open(self.bus(), deadline)?))
    }

    fn dismiss_all_notifications(
        &self,
        request: DismissAllNotificationsRequest<'_>,
        lease: &InteractionLease,
    ) -> Result<(Vec<NotificationInfo>, Vec<String>), AdapterError> {
        notifications::dismiss_all(self.bus(), request, lease)
    }

    fn dismiss_notification(
        &self,
        request: DismissNotificationRequest<'_>,
        lease: &InteractionLease,
    ) -> Result<NotificationInfo, AdapterError> {
        notifications::dismiss(self.bus(), request, lease)
    }

    fn check_notification_support(&self, deadline: Deadline) -> Result<(), AdapterError> {
        shell::detect::require_daemon(deadline)
    }

    fn list_notifications(
        &self,
        filter: &NotificationFilter,
        policy: InteractionPolicy,
        deadline: Deadline,
        lease: Option<&InteractionLease>,
    ) -> Result<Vec<NotificationInfo>, AdapterError> {
        notifications::list(self.bus(), filter, policy, deadline, lease)
    }

    fn notification_action(
        &self,
        request: NotificationActionRequest<'_>,
        lease: &InteractionLease,
    ) -> Result<ActionResult, AdapterError> {
        notifications::action(self.bus(), request, lease)
    }

    fn open_system_surface(
        &self,
        surface: SnapshotSurface,
        policy: InteractionPolicy,
        lease: &InteractionLease,
    ) -> Result<WindowInfo, AdapterError> {
        shell::open_surface(self.bus(), surface, policy, lease)
    }
}
