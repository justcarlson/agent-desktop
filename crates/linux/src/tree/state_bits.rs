pub(crate) const BUSY: u32 = 3;
pub(crate) const CHECKED: u32 = 4;
pub(crate) const EDITABLE: u32 = 7;
pub(crate) const ENABLED: u32 = 8;
pub(crate) const EXPANDED: u32 = 10;
pub(crate) const FOCUSED: u32 = 12;
pub(crate) const MODAL: u32 = 16;
pub(crate) const MULTISELECTABLE: u32 = 18;
pub(crate) const PRESSED: u32 = 20;
pub(crate) const SELECTED: u32 = 23;
pub(crate) const SENSITIVE: u32 = 24;
#[cfg(test)]
pub(crate) const SHOWING: u32 = 25;
pub(crate) const INDETERMINATE: u32 = 32;
pub(crate) const REQUIRED: u32 = 33;
pub(crate) const INVALID_ENTRY: u32 = 36;
pub(crate) const HAS_POPUP: u32 = 42;
pub(crate) const READ_ONLY: u32 = 43;

/// The AT-SPI state set as the bus delivers it: bit `n` is bit `n % 32` of
/// word `n / 32`, the low word first.
#[derive(Debug, Clone, Copy)]
pub(crate) struct StateBits<'a>(&'a [u32]);

impl<'a> StateBits<'a> {
    pub(crate) fn new(words: &'a [u32]) -> Self {
        Self(words)
    }

    pub(crate) fn has(&self, bit: u32) -> bool {
        self.0
            .get((bit / 32) as usize)
            .is_some_and(|word| word & (1 << (bit % 32)) != 0)
    }
}
