//! The display answer on a Wayland session: Mutter's layout on GNOME, the
//! compositor's output globals anywhere else, and one refusal when neither
//! source exists.

use std::path::Path;

use agent_desktop_core::{AdapterError, Deadline, DisplayInfo, ErrorCode};

use super::display_outputs;
use super::output_probe::OutputProbe;
use super::wayland_output::WaylandOutput;
use crate::system::display::display_mutter::{self, LogicalMonitor};
use crate::system::display::display_server::session_wayland_socket;
use crate::system::display::{from_mutter, primaries_first};

pub(crate) fn wayland_missing() -> AdapterError {
    AdapterError::new(
        ErrorCode::PlatformNotSupported,
        "Displays cannot be listed on this Wayland session",
    )
    .with_suggestion(
        "Run under GNOME, whose compositor answers the Mutter DisplayConfig \
         interface, or under a compositor that advertises wl_output and \
         zxdg_output_manager_v1, such as Sway or Hyprland",
    )
}

fn probe_error(probe: OutputProbe, socket: &Path) -> AdapterError {
    match probe {
        OutputProbe::Unadvertised(missing) => wayland_missing().with_platform_detail(format!(
            "the compositor does not advertise {}",
            missing.join(" or ")
        )),
        OutputProbe::Incomplete => wayland_missing()
            .with_platform_detail("an output came back without a name, position or size"),
        OutputProbe::Wayland(fault) => fault.into_error(socket),
    }
}

/// The displays of a compositor's outputs. Wayland names no primary output,
/// so the one at the layout origin is primary, else the top-left one, and
/// the lowest name breaks a tie between mirrored outputs. An
/// output without a name, position or size makes the whole list unusable,
/// because `--screen` indexes would shift under it.
pub(crate) fn from_outputs(outputs: &[WaylandOutput]) -> Option<Vec<DisplayInfo>> {
    let mut displays = outputs
        .iter()
        .map(WaylandOutput::display)
        .collect::<Option<Vec<_>>>()?;
    let at_origin = |display: &DisplayInfo| display.bounds.y == 0.0 && display.bounds.x == 0.0;
    let primary = (0..displays.len()).min_by(|left, right| {
        let (left, right) = (&displays[*left], &displays[*right]);
        at_origin(right)
            .cmp(&at_origin(left))
            .then(left.bounds.y.total_cmp(&right.bounds.y))
            .then(left.bounds.x.total_cmp(&right.bounds.x))
            .then_with(|| left.id.cmp(&right.id))
    });
    if let Some(index) = primary {
        displays[index].is_primary = true;
    }
    primaries_first(&mut displays);
    Some(displays)
}

/// The Wayland answer. Mutter's list wins where it answers, so GNOME keeps
/// its own layout; only a session without Mutter asks `outputs`.
pub(crate) fn decide_wayland(
    mutter: Option<Vec<LogicalMonitor>>,
    outputs: impl FnOnce() -> Result<Vec<DisplayInfo>, AdapterError>,
) -> Result<Vec<DisplayInfo>, AdapterError> {
    match mutter {
        Some(logical) => Ok(from_mutter(logical)),
        None => outputs(),
    }
}

/// The displays of the compositor behind `socket`, read over `wl_output` and
/// `zxdg_output_manager_v1`, or the refusal its capability probe ends in.
pub(crate) fn list_outputs(
    socket: &Path,
    deadline: Deadline,
) -> Result<Vec<DisplayInfo>, AdapterError> {
    let outputs = display_outputs::read_outputs(socket, deadline)
        .map_err(|probe| probe_error(probe, socket))?;
    from_outputs(&outputs).ok_or_else(|| probe_error(OutputProbe::Incomplete, socket))
}

/// The displays of a Wayland session, with no look at which server the
/// environment reaches: a caller that already chose the Wayland route asks
/// here. Mutter answers on GNOME; any other compositor is read over its
/// output globals.
pub(crate) fn list_wayland_displays(deadline: Deadline) -> Result<Vec<DisplayInfo>, AdapterError> {
    decide_wayland(
        display_mutter::logical_monitors(deadline)?,
        || match session_wayland_socket() {
            Some(socket) => list_outputs(&socket, deadline),
            None => Err(wayland_missing()),
        },
    )
}

#[cfg(test)]
#[path = "display_wayland_tests.rs"]
mod tests;
