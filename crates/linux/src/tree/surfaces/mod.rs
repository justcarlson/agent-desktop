use self::top_level::is_dialog_role;
use super::RawNode;
use super::vocabulary::state_bits::{MODAL, SHOWING, StateBits};

/// A top-level is a sheet when it carries MODAL or has a dialog role and
/// SHOWING. Unread role or state evidence does not establish a sheet.
pub(crate) fn is_sheet(node: &RawNode) -> bool {
    let (Some(&role), Some(words)) = (node.traits.role.present(), node.traits.states.present())
    else {
        return false;
    };
    let bits = StateBits::new(words);
    bits.has(MODAL) || (is_dialog_role(Some(&role)) && bits.has(SHOWING))
}

pub(super) mod modal_gate;
pub(crate) mod nested_dialog;
#[cfg(test)]
#[path = "surfaces_tests.rs"]
mod tests;
pub(crate) mod top_level;
pub(crate) mod twin_frames;
pub(crate) mod window_extents;
pub(crate) mod window_tie;
