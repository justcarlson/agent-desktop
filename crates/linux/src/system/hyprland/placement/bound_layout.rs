use agent_desktop_core::DisplayInfo;

use crate::system::hyprland::clients::desktop_state::DesktopState;
use crate::system::hyprland::clients::hypr_client::HyprClient;

/// The bound toplevel an element belongs to, the outputs `list-displays`
/// reports, and what the compositor shows, all read for one placement.
pub(crate) struct BoundLayout<'a> {
    pub(crate) client: &'a HyprClient,
    pub(crate) displays: &'a [DisplayInfo],
    pub(crate) desktop: &'a DesktopState,
}
