use agent_desktop_core::{
    AdapterError, FocusOutcome, InteractionLease, InteractionPolicy, WindowInfo,
};

use super::hypr_focus::focus_reported;
use super::live_hypr_world::LiveHyprWorld;
use crate::system::SharedBus;
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
