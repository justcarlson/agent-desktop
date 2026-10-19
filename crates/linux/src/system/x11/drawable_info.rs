use super::map_state::MapState;
use crate::tree::Extents;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DrawableInfo {
    pub(crate) rect: Extents,
    pub(crate) parent: Option<u32>,
    pub(crate) depth: u8,
    pub(crate) map_state: MapState,
    pub(crate) override_redirect: bool,
}
