use wayland_protocols::ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1;
use wayland_protocols_wlr::foreign_toplevel::v1::client::zwlr_foreign_toplevel_handle_v1::ZwlrForeignToplevelHandleV1;

use super::advertised_global::AdvertisedGlobal;
use super::ext_record::ExtRecord;
use super::wlr_record::WlrRecord;

/// Everything one toplevel connection has been told: the registry globals,
/// how many sync callbacks completed, and every toplevel handle of both
/// foreign toplevel protocols with what it reported.
#[derive(Default)]
pub(crate) struct ToplevelState {
    pub(crate) globals: Vec<AdvertisedGlobal>,
    pub(crate) synced: u32,
    pub(crate) ext: Vec<(ExtForeignToplevelHandleV1, ExtRecord)>,
    pub(crate) wlr: Vec<(ZwlrForeignToplevelHandleV1, WlrRecord)>,
}

impl ToplevelState {
    /// Whether the mapping manager answered for the first `requested`
    /// handles of each protocol. A handle announced after its mapping
    /// request was sent waits for the next settle.
    pub(crate) fn mapped(&self, requested: (usize, usize)) -> bool {
        let (ext, wlr) = requested;
        self.ext.iter().take(ext).all(|(_, record)| record.answered)
            && self.wlr.iter().take(wlr).all(|(_, record)| record.answered)
    }
}

#[cfg(test)]
#[path = "toplevel_state_tests.rs"]
mod tests;
