use super::RawNode;
use super::role_numbers::DIALOG;
use super::state_bits::{MODAL, SHOWING, StateBits};

/// Whether the window root is a sheet: a dialog that is showing, or any
/// top-level that carries `MODAL`. GTK 3 presents its About window as a
/// showing dialog with no `MODAL`; GTK 4 presents the same window as a modal
/// frame. A role or a state set that never read is not a sheet: absent
/// evidence stays absent.
pub(crate) fn is_sheet(node: &RawNode) -> bool {
    let (Some(&role), Some(words)) = (node.traits.role.present(), node.traits.states.present())
    else {
        return false;
    };
    let bits = StateBits::new(words);
    bits.has(MODAL) || (role == DIALOG && bits.has(SHOWING))
}

#[cfg(test)]
#[path = "surfaces_tests.rs"]
mod tests;
