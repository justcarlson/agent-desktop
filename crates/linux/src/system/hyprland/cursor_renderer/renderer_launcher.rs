use agent_desktop_core::{AdapterError, CursorOverlayControl};

use super::renderer_paths::RendererPaths;
use super::renderer_pidfd::RendererPidfd;
use super::renderer_target::RendererTarget;

/// What starting a renderer needs from its surroundings, so the start lock
/// and record rules are testable without a compositor.
pub(crate) trait RendererLauncher {
    /// Whether the session still wants a cursor for `control`.
    fn session_active(&self, control: &CursorOverlayControl) -> bool;

    /// Starts one renderer for `target` that will listen on `paths.socket`,
    /// handing it `first` as its startup control. Returns a pidfd for the
    /// started process when there is one, so a start that times out can
    /// stop exactly that process.
    fn launch(
        &self,
        target: &RendererTarget,
        paths: &RendererPaths,
        first: &CursorOverlayControl,
    ) -> Result<Option<RendererPidfd>, AdapterError>;
}
