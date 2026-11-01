use agent_desktop_core::Rect;

/// One layer surface from `j/layers`: its namespace, owning process,
/// global layout rectangle, layer level (0 background to 3 overlay) and
/// the output that holds it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LayerInfo {
    pub(crate) namespace: String,
    pub(crate) pid: Option<u32>,
    pub(crate) rect: Rect,
    pub(crate) level: u8,
    pub(crate) output: String,
}
