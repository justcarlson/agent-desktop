//! Joins each foreign toplevel's identifier to the Hyprland address the
//! mapping manager gave its handle. A handle the mapping manager could not
//! place, or one with no identifier, names no window and is left out. Two
//! live handles for one address mean the read cannot tell windows apart, so
//! the whole read is unusable.

use std::collections::HashSet;

use super::toplevel_view::ToplevelView;

/// One view per placed handle, in the compositor's announce order. Each
/// record is a handle's identifier and the address it was mapped to.
pub(crate) fn join(records: &[(String, Option<u64>)]) -> Option<Vec<ToplevelView>> {
    let placed: Vec<ToplevelView> = records
        .iter()
        .filter(|(identifier, _)| !identifier.is_empty())
        .filter_map(|(identifier, address)| {
            Some(ToplevelView {
                identifier: identifier.clone(),
                address: (*address)?,
            })
        })
        .collect();
    let mut seen = HashSet::new();
    placed
        .iter()
        .all(|view| seen.insert(view.address))
        .then_some(placed)
}

#[cfg(test)]
#[path = "toplevel_join_tests.rs"]
mod tests;
