#![expect(
    dead_code,
    reason = "the module is laid out ahead of the code that calls it"
)]

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Keymap {
    pub(crate) min_keycode: u8,
    pub(crate) keysyms_per_keycode: u8,
    pub(crate) keysyms: Vec<u32>,
    pub(crate) modifier_codes_per_modifier: u8,
    pub(crate) modifier_codes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KeymapEdit {
    pub(crate) keycode: u8,
    pub(crate) keysyms: Vec<u32>,
}

impl Keymap {
    pub(crate) fn parse(
        min_keycode: u8,
        keysyms_per_keycode: u8,
        keysyms: Vec<u32>,
        modifier_codes_per_modifier: u8,
        modifier_codes: Vec<u8>,
    ) -> Self {
        Self {
            min_keycode,
            keysyms_per_keycode,
            keysyms,
            modifier_codes_per_modifier,
            modifier_codes,
        }
    }
}

#[cfg(test)]
#[path = "keymap_tests.rs"]
mod tests;
