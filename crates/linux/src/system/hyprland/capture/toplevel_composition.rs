//! How Hyprland composes a bound toplevel, which decides whether a capture
//! may answer with its first frame. Hyprland composes a mapped toplevel on
//! a workspace a monitor shows, and asks it for every frame, so its buffer
//! is current there. It composes no client of a workspace that a fullscreen
//! or maximized client fills except that one, and nothing on a workspace
//! no monitor shows. Those toplevels stop drawing, and their buffers grow
//! old.

use crate::system::hyprland::clients::desktop_state::DesktopState;
use crate::system::hyprland::clients::hypr_client::HyprClient;
use crate::system::wayland::capture::composition::Composition;

/// `Current` when `client` is shown, in view as `DesktopState::in_view`
/// decides, and no other client fills its workspace. `filling` is every
/// client that fills its workspace, by address and workspace name; `None`
/// means the compositor did not say, which is `Stale`.
pub(crate) fn composition_of(
    client: &HyprClient,
    desktop: &DesktopState,
    filling: Option<&[(u64, String)]>,
) -> Composition {
    let Some(filling) = filling else {
        return Composition::Stale;
    };
    let covered = filling.iter().any(|(address, workspace)| {
        *address != client.address && *workspace == client.workspace.name
    });
    if client.shown && desktop.in_view(&client.workspace) && !covered {
        Composition::Current
    } else {
        Composition::Stale
    }
}

#[cfg(test)]
#[path = "toplevel_composition_tests.rs"]
mod tests;
