use agent_desktop_core::WindowInfo;

use super::x11::FrameFacts;

/// A window of the registry with what focus needs beside what is reported.
#[derive(Debug, Clone)]
pub(crate) struct ListedWindow {
    pub(crate) app: usize,
    pub(crate) info: WindowInfo,
    pub(crate) active: bool,
    pub(crate) frame: FrameFacts,
}
