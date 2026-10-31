//! The seat owner's compiled XKB keymap, extended with only the keysyms a
//! delivery needs and the owner's layout lacks.

use agent_desktop_core::AdapterError;

/// Where one requested keysym is typed under the extended keymap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KeyPlacement {
    pub(crate) keysym: u32,
    pub(crate) xkb_keycode: u32,
    pub(crate) level: u8,
    pub(crate) added: bool,
}

/// The owner's keymap text with any added keys, and a placement for each
/// requested keysym.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExtendedKeymap {
    pub(crate) text: String,
    pub(crate) placements: Vec<KeyPlacement>,
}

pub(crate) fn extend(_owner: &str, _keysyms: &[u32]) -> Result<ExtendedKeymap, AdapterError> {
    Err(AdapterError::not_supported("owner keymap extension"))
}

#[cfg(test)]
#[path = "owner_keymap_tests.rs"]
mod tests;
