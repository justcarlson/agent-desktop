#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub(super) struct Overlays {
    pub(super) bounds: Vec<crate::tree::Extents>,
    pub(super) receiving: Vec<(u32, crate::tree::Extents)>,
    pub(super) names: Vec<(u32, String)>,
}
