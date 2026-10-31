use crate::system::hyprland::clients::desktop_state::DesktopState;
use crate::system::wayland::toplevel::toplevel_view::ToplevelView;

/// What a fake compositor reports at one moment.
#[derive(Debug, Clone)]
pub(crate) struct FakeSnapshot {
    pub(crate) desktop: DesktopState,
    pub(crate) toplevels: Vec<ToplevelView>,
    pub(crate) frame_active: bool,
}
