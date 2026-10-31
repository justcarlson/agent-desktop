//! Hyprland `focus-window`. The window is bound to exactly one toplevel
//! before anything is sent. Focus is proven by Hyprland's active window,
//! the toplevel's `activated` state, its workspace being in view and, for
//! an accessibility frame, the frame's ACTIVE state. Only a headed command activates, through
//! `zwlr_foreign_toplevel_handle_v1.activate`; a headless one proves
//! existing focus or refuses.

use agent_desktop_core::{
    AdapterError, FocusActivation, FocusOutcome, FocusProof, InteractionPolicy, WindowInfo,
};

use super::focus_refusal::{not_focused, proof_refusal, special_open};
use super::hypr_world::HyprWorld;
use super::proof_read::ProofRead;
use crate::system::hyprland::clients::desktop_state::DesktopState;
use crate::system::hyprland::clients::hypr_client::HyprClient;
use crate::system::hyprland::clients::side_effects::side_effects;
use crate::system::hyprland::identity::compositor_window_id::is_compositor_id;
use crate::system::hyprland::identity::inventory::Inventory;
use crate::system::hyprland::identity::window_bind::bind_window;

const SPECIAL_PREFIX: &str = "special:";

/// Focuses `win` under `policy` and reports how focus was proven and what
/// else changed on the desktop.
pub(crate) fn focus_reported(
    world: &mut impl HyprWorld,
    win: &WindowInfo,
    policy: InteractionPolicy,
) -> Result<FocusOutcome, AdapterError> {
    let bound = bind(world, win)?;
    let accessible = !is_compositor_id(&win.id);
    let proof = if accessible {
        FocusProof::CompositorAndAccessibility
    } else {
        FocusProof::CompositorOnly
    };
    let (read, before) = witness(world, win, &bound, accessible)?;
    if read.agrees() {
        return Ok(outcome(win, false, proof, Vec::new()));
    }
    if let Some(covering) = before.covering_special(&bound.workspace) {
        return Err(special_open(covering));
    }
    if !policy.is_headed() {
        return Err(headless_refusal(read, &bound, &before));
    }
    world.activate(bound.address)?;
    let mut last = (read, before.clone());
    let mut last_error: Option<AdapterError>;
    loop {
        match witness(world, win, &bound, accessible) {
            Ok((read, after)) if read.agrees() => {
                return Ok(outcome(win, true, proof, side_effects(&before, &after)));
            }
            Ok(observed) => {
                last = observed;
                last_error = None;
            }
            Err(error) => last_error = Some(error),
        }
        if !world.wait_more() {
            break;
        }
    }
    Err(proof_refusal(
        last.0,
        true,
        &side_effects(&before, &last.1),
        last_error.as_ref(),
    ))
}

fn bind(world: &mut impl HyprWorld, win: &WindowInfo) -> Result<HyprClient, AdapterError> {
    let (clients, toplevels) = world.inventory()?;
    let inventory = Inventory {
        clients: &clients,
        toplevels: &toplevels,
    };
    let live = world.live_instance(win.pid.get());
    bind_window(win, inventory, live.as_deref(), world.compositor()).map(|bound| bound.client)
}

fn witness(
    world: &mut impl HyprWorld,
    win: &WindowInfo,
    bound: &HyprClient,
    accessible: bool,
) -> Result<(ProofRead, DesktopState), AdapterError> {
    let address = bound.address;
    let desktop = world.desktop()?;
    let toplevel_activated = world.activated()?.contains(&address);
    let frame_active = if accessible {
        Some(world.frame_active(win)?)
    } else {
        None
    };
    let read = ProofRead {
        compositor_active: desktop.active == Some(address),
        toplevel_activated,
        in_view: desktop.in_view(&bound.workspace),
        frame_active,
    };
    Ok((read, desktop))
}

fn headless_refusal(read: ProofRead, bound: &HyprClient, before: &DesktopState) -> AdapterError {
    let workspace = bound.workspace.name.as_str();
    if workspace.starts_with(SPECIAL_PREFIX) && !before.shows(workspace) {
        return not_focused(Some(workspace));
    }
    if read.compositor_active || read.toplevel_activated {
        return proof_refusal(read, false, &[], None);
    }
    not_focused(None)
}

fn outcome(
    win: &WindowInfo,
    activated: bool,
    proof: FocusProof,
    side_effects: Vec<agent_desktop_core::FocusSideEffect>,
) -> FocusOutcome {
    let mut window = win.clone();
    window.state.is_focused = true;
    FocusOutcome {
        window,
        activation: FocusActivation {
            activated,
            proof,
            side_effects,
        },
    }
}

#[cfg(test)]
#[path = "hypr_focus_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "hypr_focus_view_tests.rs"]
mod view_tests;
