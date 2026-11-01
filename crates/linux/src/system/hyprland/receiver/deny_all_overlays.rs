use super::layer_info::LayerInfo;
use super::overlay_filter::OverlayFilter;

/// Accepts no layer as an own cursor overlay, so every top and overlay
/// layer that holds the point is a cover.
pub(crate) struct DenyAllOverlays;

impl OverlayFilter for DenyAllOverlays {
    fn is_own_cursor(&self, _layer: &LayerInfo) -> bool {
        false
    }
}
