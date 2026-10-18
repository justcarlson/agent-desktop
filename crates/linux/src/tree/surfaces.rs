use super::RawNode;
use super::state_bits::{MODAL, StateBits};

const DIALOG: u32 = 16;

/// Whether the window root is a modal dialog, which this platform presents
/// as the sheet surface. A role or a state set that never read is not a
/// sheet: absent evidence stays absent.
pub(crate) fn is_sheet(node: &RawNode) -> bool {
    node.traits.role.present() == Some(&DIALOG)
        && node
            .traits
            .states
            .present()
            .is_some_and(|words| StateBits::new(words).has(MODAL))
}

#[cfg(test)]
#[path = "surfaces_tests.rs"]
mod tests;
