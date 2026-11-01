use super::layer_info::LayerInfo;

/// Decides whether a layer surface is this tool's own cursor overlay, which
/// takes no input and so covers nothing. A namespace alone is no proof: any
/// client can name its surface `agent-desktop-cursor`.
pub(crate) trait OverlayFilter {
    fn is_own_cursor(&self, layer: &LayerInfo) -> bool;
}
