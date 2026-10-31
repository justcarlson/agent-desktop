//! The globals a toplevel connection binds, chosen from the registry.

use super::advertised_global::AdvertisedGlobal;

/// The registry name and bind version of each global a toplevel session
/// uses. The seat is optional because only activation needs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChosenGlobals {
    pub(crate) ext_list: AdvertisedGlobal,
    pub(crate) wlr_manager: AdvertisedGlobal,
    pub(crate) mapping: AdvertisedGlobal,
    pub(crate) seat: Option<AdvertisedGlobal>,
}

const EXT_LIST: (&str, u32) = ("ext_foreign_toplevel_list_v1", 1);
const WLR_MANAGER: (&str, u32) = ("zwlr_foreign_toplevel_manager_v1", 3);
const MAPPING: (&str, u32) = ("hyprland_toplevel_mapping_manager_v1", 1);
const SEAT: (&str, u32) = ("wl_seat", 1);

/// The toplevel globals at the lower of the advertised version and the
/// version this client speaks, or the interface name of the first one the
/// compositor does not advertise at a usable version.
pub(crate) fn choose(globals: &[AdvertisedGlobal]) -> Result<ChosenGlobals, &'static str> {
    let required = |(interface, speaks): (&'static str, u32)| {
        pick(globals, interface, speaks).ok_or(interface)
    };
    Ok(ChosenGlobals {
        ext_list: required(EXT_LIST)?,
        wlr_manager: required(WLR_MANAGER)?,
        mapping: required(MAPPING)?,
        seat: pick(globals, SEAT.0, SEAT.1),
    })
}

/// The first global named `interface` at the lower of its advertised version
/// and `speaks`.
pub(crate) fn pick(
    globals: &[AdvertisedGlobal],
    interface: &str,
    speaks: u32,
) -> Option<AdvertisedGlobal> {
    globals
        .iter()
        .find(|global| global.interface == interface && global.version >= 1)
        .map(|global| AdvertisedGlobal {
            version: global.version.min(speaks),
            ..global.clone()
        })
}

#[cfg(test)]
#[path = "chosen_globals_tests.rs"]
mod tests;
