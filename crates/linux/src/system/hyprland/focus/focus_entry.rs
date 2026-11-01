use agent_desktop_core::{
    AdapterError, FocusOutcome, InteractionLease, InteractionPolicy, WindowInfo,
};

use super::hypr_focus::focus_reported;
use super::live_hypr_world::LiveHyprWorld;
use crate::system::SharedBus;
use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;
use crate::system::hyprland::bind::routed_binding::routed_binding;

const METHOD: &str = "focus_window_reported";

/// Hyprland `focus-window` with its proof and side effects. Any other route
/// answers the default `not_supported`, so the caller uses `focus_window`.
pub(crate) fn focus_window_reported(
    bus: &SharedBus,
    win: &WindowInfo,
    policy: InteractionPolicy,
    lease: &InteractionLease,
) -> Result<FocusOutcome, AdapterError> {
    let Some(binding) = routed_binding()? else {
        return Err(AdapterError::not_supported(METHOD));
    };
    let mut world = LiveHyprWorld::open(binding, bus, lease.deadline())?;
    focus_reported(&mut world, win, policy)
}

/// Headed focus for an action on a bound Hyprland instance: the compositor
/// activates the window and both it and the accessibility tree must agree.
pub(crate) fn focus_bound(
    binding: HyprlandBinding,
    bus: &SharedBus,
    win: &WindowInfo,
    lease: &InteractionLease,
) -> Result<(), AdapterError> {
    let mut world = LiveHyprWorld::open(binding, bus, lease.deadline())?;
    focus_reported(&mut world, win, InteractionPolicy::headed()).map(|_| ())
}
