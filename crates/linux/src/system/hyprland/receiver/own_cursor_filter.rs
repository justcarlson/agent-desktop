//! The overlay filter for a command that holds a cursor hide: a
//! `agent-desktop-cursor` layer passes only when its process acknowledged
//! this command's hide, so it shows a transparent frame and, with its empty
//! input region, covers nothing. A layer that merely carries the namespace
//! still covers.

use super::deny_all_overlays::DenyAllOverlays;
use super::layer_info::LayerInfo;
use super::overlay_filter::OverlayFilter;
use crate::system::hyprland::cursor_renderer::cursor_surface::CURSOR_NAMESPACE;
use crate::system::hyprland::cursor_renderer::hold_registry::acknowledged_renderers;

/// Accepts the cursor layers of the renderers in `renderers`.
pub(crate) struct OwnCursorFilter {
    renderers: Vec<u32>,
}

impl OwnCursorFilter {
    pub(crate) fn of(renderers: Vec<u32>) -> Self {
        Self { renderers }
    }

    /// The filter for the calling thread's hide holds; with none, every
    /// layer covers.
    pub(crate) fn current() -> Box<dyn OverlayFilter> {
        let renderers = acknowledged_renderers();
        if renderers.is_empty() {
            return Box::new(DenyAllOverlays);
        }
        Box::new(Self::of(renderers))
    }
}

impl OverlayFilter for OwnCursorFilter {
    fn is_own_cursor(&self, layer: &LayerInfo) -> bool {
        layer.namespace == CURSOR_NAMESPACE
            && layer.pid.is_some_and(|pid| self.renderers.contains(&pid))
    }
}

#[cfg(test)]
#[path = "own_cursor_filter_tests.rs"]
mod tests;
