use crate::system::hyprland::clients::desktop_state::DesktopState;

/// What a fake compositor reports at one moment.
#[derive(Debug, Clone)]
pub(crate) struct FakeSnapshot {
    pub(crate) desktop: DesktopState,
    pub(crate) activated: Vec<u64>,
    pub(crate) frame_active: bool,
}
