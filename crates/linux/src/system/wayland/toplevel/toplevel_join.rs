//! Joins the two foreign toplevel protocols by the Hyprland address the
//! mapping manager gave each handle. A handle the mapping manager could not
//! place, or an ext handle with no identifier, names no window and is left
//! out. Two live handles of one protocol for one address mean the read
//! cannot tell windows apart, so the whole read is unusable.

use std::collections::HashSet;

use super::ext_record::ExtRecord;
use super::toplevel_view::ToplevelView;
use super::wlr_record::WlrRecord;

/// One view per placed ext handle, in the compositor's announce order.
pub(crate) fn join(ext: &[ExtRecord], wlr: &[WlrRecord]) -> Option<Vec<ToplevelView>> {
    let live_wlr: Vec<(usize, u64, bool)> = wlr
        .iter()
        .enumerate()
        .filter(|(_, record)| !record.closed)
        .filter_map(|(index, record)| Some((index, record.address?, record.activated)))
        .collect();
    if !distinct(live_wlr.iter().map(|(_, address, _)| *address)) {
        return None;
    }
    let placed: Vec<(&str, u64)> = ext
        .iter()
        .filter(|record| !record.identifier.is_empty())
        .filter_map(|record| Some((record.identifier.as_str(), record.address?)))
        .collect();
    if !distinct(placed.iter().map(|(_, address)| *address)) {
        return None;
    }
    Some(
        placed
            .into_iter()
            .map(|(identifier, address)| {
                let handle = live_wlr
                    .iter()
                    .find(|(_, wlr_address, _)| *wlr_address == address);
                ToplevelView {
                    identifier: identifier.to_owned(),
                    address,
                    activated: handle.is_some_and(|(_, _, activated)| *activated),
                    wlr_index: handle.map(|(index, _, _)| *index),
                }
            })
            .collect(),
    )
}

fn distinct(addresses: impl Iterator<Item = u64>) -> bool {
    let mut seen = HashSet::new();
    addresses.into_iter().all(|address| seen.insert(address))
}

#[cfg(test)]
#[path = "toplevel_join_tests.rs"]
mod tests;
