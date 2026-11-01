use std::collections::HashMap;
use std::collections::hash_map::Entry;

use agent_desktop_core::AdapterError;

use super::evdev_key::EvdevKey;
use super::press_state::PressState;
use super::refusal::keymap_refused;
use super::xkb_keymap::XkbKeymap;
use crate::input::keyboard::key_names::{KEYSYM_SHIFT_L, keysym_for_char};

pub(crate) const EVDEV_OFFSET: u32 = 8;
const KEYSYM_SHIFT_R: u32 = 0xffe2;
const LEVEL_PLAIN: u32 = 0;
const LEVEL_SHIFT: u32 = 1;

/// Where each keysym of a keymap is typed in one layout: the lowest key that
/// gives it on Level1 with no modifier, else on Level2 with Shift alone. A
/// keysym that only another level (AltGr, Level5) or another layout gives is
/// absent. A legacy keysym such as `Cyrillic_a` also answers for the
/// Unicode-form keysym that text input sends for its character, unless a key
/// gives that keysym itself.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct KeyTable {
    keys: HashMap<u32, EvdevKey>,
}

impl KeyTable {
    /// Reads XKB keymap text, as a compositor sends it, for layout `layout`.
    pub(crate) fn from_keymap(keymap: &[u8], layout: u32) -> Result<Self, AdapterError> {
        let text = keymap.strip_suffix(&[0]).unwrap_or(keymap);
        let compiled = XkbKeymap::compile(text)?.ok_or_else(|| {
            keymap_refused(
                "The keyboard keymap of this Wayland session did not compile",
                "keymap_invalid",
                serde_json::json!({}),
            )
        })?;
        Self::from_compiled(&compiled, layout)
    }

    /// Reads an already compiled keymap for layout `layout`. A key with fewer
    /// layouts wraps the index, as XKB does when it resolves a key.
    pub(crate) fn from_compiled(keymap: &XkbKeymap, layout: u32) -> Result<Self, AdapterError> {
        let shift_mask = keymap.mod_mask(c"Shift")?;
        let mut keys = HashMap::new();
        let mut aliases = HashMap::new();
        let range = keymap.keycodes()?;
        for keycode in (*range.start()).max(EVDEV_OFFSET)..=*range.end() {
            let layouts = keymap.layouts_for_key(keycode)?;
            if layouts == 0 {
                continue;
            }
            let group = layout % layouts;
            let levels = keymap.levels_for_key(keycode, group)?;
            for (level, shift, mask) in [(LEVEL_PLAIN, false, 0), (LEVEL_SHIFT, true, shift_mask)] {
                if level >= levels || (shift && mask == 0) {
                    continue;
                }
                if !keymap
                    .masks_for_level(keycode, group, level)?
                    .contains(&mask)
                {
                    continue;
                }
                let key = EvdevKey {
                    code: keycode - EVDEV_OFFSET,
                    shift,
                };
                for keysym in keymap.syms_by_level(keycode, group, level)? {
                    place(&mut keys, keysym, key);
                    if let Some(character) = keymap.keysym_char(keysym)? {
                        place(&mut aliases, keysym_for_char(character), key);
                    }
                }
            }
        }
        for (keysym, key) in aliases {
            keys.entry(keysym).or_insert(key);
        }
        Ok(Self { keys })
    }

    pub(crate) fn lookup(&self, keysym: u32) -> Option<EvdevKey> {
        self.keys.get(&keysym).copied()
    }

    /// The evdev edges that send `keysym` going `down` or up. A keysym on the
    /// Shift level goes inside a Shift press of its own unless the caller
    /// already holds Shift; its release lets that Shift go after the key.
    pub(crate) fn plan(
        &self,
        keysym: u32,
        down: bool,
        state: &mut PressState,
    ) -> Option<Vec<(u32, bool)>> {
        let key = self.lookup(keysym)?;
        if matches!(keysym, KEYSYM_SHIFT_L | KEYSYM_SHIFT_R) {
            state.explicit_shift = if down {
                state.explicit_shift + 1
            } else {
                state.explicit_shift.saturating_sub(1)
            };
            return Some(vec![(key.code, down)]);
        }
        let shift = self.lookup(KEYSYM_SHIFT_L)?.code;
        if down {
            if key.shift && state.explicit_shift == 0 {
                state.wrapped.push(key.code);
                return Some(vec![(shift, true), (key.code, true)]);
            }
            return Some(vec![(key.code, true)]);
        }
        let Some(at) = state.wrapped.iter().rposition(|code| *code == key.code) else {
            return Some(vec![(key.code, false)]);
        };
        state.wrapped.remove(at);
        Some(vec![(key.code, false), (shift, false)])
    }
}

fn place(keys: &mut HashMap<u32, EvdevKey>, keysym: u32, key: EvdevKey) {
    match keys.entry(keysym) {
        Entry::Vacant(slot) => {
            slot.insert(key);
        }
        Entry::Occupied(mut slot) if slot.get().shift && !key.shift => {
            slot.insert(key);
        }
        Entry::Occupied(_) => {}
    }
}

#[cfg(test)]
#[path = "key_table_tests.rs"]
mod tests;
