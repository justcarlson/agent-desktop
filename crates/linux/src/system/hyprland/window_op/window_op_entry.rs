use agent_desktop_core::{
    AdapterError, DeliverySemantics, InteractionLease, OperationReport, WindowInfo, WindowOp,
};

use super::hypr_window_op::window_op_reported;
use super::live_window_world::LiveWindowWorld;
use crate::system::hyprland::bind::routed_binding::routed_binding;

/// The Hyprland window operation when this session's display route is
/// Hyprland, `None` on any other route, which keeps its own engine.
pub(crate) fn hyprland_window_op(
    win: &WindowInfo,
    op: &WindowOp,
    lease: &InteractionLease,
) -> Result<Option<OperationReport>, AdapterError> {
    let binding = routed_binding()
        .map_err(|error| error.with_disposition(DeliverySemantics::not_delivered()))?;
    let Some(binding) = binding else {
        return Ok(None);
    };
    let mut world = LiveWindowWorld::new(binding, lease.deadline());
    window_op_reported(&mut world, win, op).map(Some)
}
