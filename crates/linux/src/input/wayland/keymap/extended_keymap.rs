use super::key_placement::KeyPlacement;
use super::key_table::KeyTable;

/// The owner's keymap text with any added keys, a placement for each
/// requested keysym, and the key table of the extended keymap in the active
/// layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExtendedKeymap {
    pub(crate) text: String,
    pub(crate) placements: Vec<KeyPlacement>,
    pub(crate) table: KeyTable,
}
