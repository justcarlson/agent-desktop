#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the planner is not called until the transport is wired"
    )
)]

use agent_desktop_core::Modifier;

const SHIFT_ROW: usize = 0;
const CONTROL_ROW: usize = 2;
const ALT_ROW: usize = 3;
const SUPER_ROW: usize = 6;
const GROUP_ONE_LEVELS: usize = 2;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct KeyLookup {
    pub(crate) code: u8,
    pub(crate) shift: bool,
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

    fn width(&self) -> usize {
        usize::from(self.keysyms_per_keycode)
    }

    fn keycode_count(&self) -> usize {
        match self.width() {
            0 => 0,
            width => self.keysyms.len() / width,
        }
    }

    fn code_at(&self, index: usize) -> Option<u8> {
        u8::try_from(index + usize::from(self.min_keycode)).ok()
    }

    fn column(&self, index: usize, level: usize) -> Option<u32> {
        if level >= self.width() {
            return None;
        }
        self.keysyms.get(index * self.width() + level).copied()
    }

    pub(crate) fn lookup(&self, keysym: u32) -> Option<KeyLookup> {
        if keysym == 0 {
            return None;
        }
        (0..GROUP_ONE_LEVELS).find_map(|level| {
            (0..self.keycode_count()).find_map(|index| {
                (self.column(index, level) == Some(keysym))
                    .then(|| self.code_at(index))
                    .flatten()
                    .map(|code| KeyLookup {
                        code,
                        shift: level == 1,
                    })
            })
        })
    }

    pub(crate) fn modifier_code(&self, modifier: &Modifier) -> Option<u8> {
        let row = match modifier {
            Modifier::Shift => SHIFT_ROW,
            Modifier::Ctrl => CONTROL_ROW,
            Modifier::Alt => ALT_ROW,
            Modifier::Meta => SUPER_ROW,
        };
        let width = usize::from(self.modifier_codes_per_modifier);
        self.modifier_codes
            .get(row * width..(row + 1) * width)?
            .iter()
            .copied()
            .find(|code| *code != 0)
    }

    pub(crate) fn spare_keycode(&self) -> Option<u8> {
        (0..self.keycode_count())
            .rev()
            .find(|index| (0..self.width()).all(|level| self.column(*index, level) == Some(0)))
            .and_then(|index| self.code_at(index))
    }

    pub(crate) fn keysyms_of(&self, keycode: u8) -> Option<Vec<u32>> {
        let index = usize::from(keycode).checked_sub(usize::from(self.min_keycode))?;
        (index < self.keycode_count()).then(|| {
            (0..self.width())
                .filter_map(|level| self.column(index, level))
                .collect()
        })
    }
}

#[cfg(test)]
impl Keymap {
    pub(crate) fn sample() -> Self {
        #[rustfmt::skip]
        let keysyms = vec![
            0xffe1, 0,    0, 0,
            0xffe3, 0,    0, 0,
            0x61,   0x41, 0, 0,
            0x31,   0x21, 0, 0,
            0,      0,    0, 0,
            0xe9,   0xc9, 0, 0,
            0,      0,    0, 0,
        ];
        let mut modifier_codes = vec![0u8; 16];
        modifier_codes[0] = 8;
        modifier_codes[4] = 9;
        modifier_codes[7] = 12;
        modifier_codes[13] = 15;
        Self::parse(8, 4, keysyms, 2, modifier_codes)
    }
}

#[cfg(test)]
#[path = "keymap_tests.rs"]
mod tests;
