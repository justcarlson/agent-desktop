//! The toplevel globals of the scripted compositor: an
//! `ext_foreign_toplevel_list_v1` that announces each fake toplevel with its
//! identifier, and a `hyprland_toplevel_mapping_manager_v1` that answers
//! each mapping request with the toplevel's address or `failed`.

use super::fake_compositor::{Arg, message};

pub(super) const LIST: &str = "ext_foreign_toplevel_list_v1";
pub(super) const MAPPER: &str = "hyprland_toplevel_mapping_manager_v1";

const FIRST_SERVER_ID: u32 = 0xff00_0000;

/// One toplevel the fake compositor announces. `address` is what its
/// mapping request answers; `None` answers `failed`.
#[derive(Debug, Clone)]
pub(crate) struct FakeToplevel {
    pub(crate) identifier: &'static str,
    pub(crate) address: Option<u64>,
}

fn handle_id(index: usize) -> u32 {
    FIRST_SERVER_ID + u32::try_from(index).expect("few toplevels")
}

/// The list's `toplevel` event for each toplevel, then its `identifier`
/// and `done` events.
pub(super) fn list_events(list: u32, toplevels: &[FakeToplevel]) -> Vec<u8> {
    let mut out = Vec::new();
    for (index, toplevel) in toplevels.iter().enumerate() {
        let id = handle_id(index);
        out.extend(message(list, 0, &[Arg::Uint(id)]));
        out.extend(message(id, 4, &[Arg::Text(toplevel.identifier)]));
        out.extend(message(id, 1, &[]));
    }
    out
}

/// The answer to `get_window_for_toplevel(handle, toplevel)`.
pub(super) fn mapping_reply(handle: u32, toplevel: u32, toplevels: &[FakeToplevel]) -> Vec<u8> {
    let address = toplevel
        .checked_sub(FIRST_SERVER_ID)
        .and_then(|index| toplevels.get(index as usize))
        .and_then(|toplevel| toplevel.address);
    match address {
        Some(address) => {
            let high = u32::try_from(address >> 32).expect("upper half");
            let low = u32::try_from(address & 0xffff_ffff).expect("lower half");
            message(handle, 0, &[Arg::Uint(high), Arg::Uint(low)])
        }
        None => message(handle, 1, &[]),
    }
}
