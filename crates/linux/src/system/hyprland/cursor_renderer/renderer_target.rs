use std::path::PathBuf;

use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;

/// The compositor a renderer draws on and the runtime dir its registry lives
/// under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RendererTarget {
    pub(crate) binding: HyprlandBinding,
    pub(crate) runtime_dir: PathBuf,
}
