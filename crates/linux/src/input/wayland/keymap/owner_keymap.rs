//! The seat owner's compiled XKB keymap, extended with only the keysyms a
//! delivery needs and the owner's active layout lacks. A keymap built for the
//! text alone loses toolkit bindings such as Tab, so the owner's layout,
//! variant and options stay as the compositor compiled them.

use agent_desktop_core::AdapterError;
use serde_json::json;

use super::added_key::AddedKey;
use super::evdev_key::EvdevKey;
use super::extended_keymap::ExtendedKeymap;
use super::key_placement::KeyPlacement;
use super::key_table::{EVDEV_OFFSET, KeyTable};
use super::keymap_text::insert_keys;
use super::refusal::keymap_refused;
use super::xkb_keymap::XkbKeymap;

const FIRST_FREE_KEYCODE: u32 = EVDEV_OFFSET + 1;
const LAST_FREE_KEYCODE: u32 = 255;
const NAME_PREFIXES: [&str; 3] = ["I", "ADI", "ADK"];

/// Extends `owner` so every keysym in `keysyms` is typed in layout `layout`.
/// A keysym the owner already gives on Level1, or on Level2 with Shift alone,
/// keeps that key. Each other keysym gets a ONE_LEVEL key on a keycode the
/// owner leaves unbound. Every owner line stays as it was.
pub(crate) fn extend(
    owner: &str,
    keysyms: &[u32],
    layout: u32,
) -> Result<ExtendedKeymap, AdapterError> {
    let owner = owner.strip_suffix('\0').unwrap_or(owner);
    let compiled = XkbKeymap::compile(owner.as_bytes())?.ok_or_else(|| {
        keymap_refused(
            "The keyboard keymap of this Wayland session did not compile",
            "owner_keymap_invalid",
            json!({}),
        )
    })?;
    let owned = KeyTable::from_compiled(&compiled, layout)?;
    let mut placements: Vec<KeyPlacement> = Vec::new();
    let mut missing: Vec<u32> = Vec::new();
    for keysym in keysyms.iter().copied() {
        if placements
            .iter()
            .any(|placement| placement.keysym == keysym)
            || missing.contains(&keysym)
        {
            continue;
        }
        match owned.lookup(keysym) {
            Some(key) => placements.push(owner_placement(keysym, key)),
            None => missing.push(keysym),
        }
    }
    let added = add_keys(&compiled, &missing)?;
    for (keysym, key) in missing.iter().zip(&added) {
        placements.push(KeyPlacement {
            keysym: *keysym,
            xkb_keycode: key.keycode,
            level: 1,
            added: true,
        });
    }
    let text = insert_keys(owner, &added).ok_or_else(|| {
        keymap_refused(
            "The keyboard keymap of this Wayland session has an unrecognized layout",
            "owner_keymap_unrecognized",
            json!({}),
        )
    })?;
    let table = verified_table(&text, &placements, layout)?;
    Ok(ExtendedKeymap {
        text,
        placements,
        table,
    })
}

fn owner_placement(keysym: u32, key: EvdevKey) -> KeyPlacement {
    KeyPlacement {
        keysym,
        xkb_keycode: key.code + EVDEV_OFFSET,
        level: if key.shift { 2 } else { 1 },
        added: false,
    }
}

fn add_keys(keymap: &XkbKeymap, missing: &[u32]) -> Result<Vec<AddedKey>, AdapterError> {
    if missing.is_empty() {
        return Ok(Vec::new());
    }
    let mut symbols = Vec::with_capacity(missing.len());
    for keysym in missing.iter().copied() {
        match keymap.keysym_name(keysym)? {
            Some(name) if keysym != 0 => symbols.push(name),
            _ => {
                return Err(keymap_refused(
                    "A requested key has no keysym the keyboard keymap can carry",
                    "keysym_invalid",
                    json!({ "keysym": keysym }),
                ));
            }
        }
    }
    let free = free_keycodes(keymap, missing.len())?;
    if free.len() < missing.len() {
        return Err(keymap_refused(
            "The keyboard keymap of this Wayland session has too few free keys for the requested keys",
            "no_free_keycode",
            json!({ "needed": missing.len(), "available": free.len() }),
        ));
    }
    Ok(free
        .into_iter()
        .zip(symbols)
        .map(|((keycode, name, named_by_owner), symbol)| AddedKey {
            keycode,
            name,
            named_by_owner,
            symbol,
        })
        .collect())
}

fn free_keycodes(
    keymap: &XkbKeymap,
    wanted: usize,
) -> Result<Vec<(u32, String, bool)>, AdapterError> {
    let range = keymap.keycodes()?;
    let first = (*range.start()).max(FIRST_FREE_KEYCODE);
    let last = (*range.end()).min(LAST_FREE_KEYCODE);
    let mut free = Vec::new();
    for keycode in first..=last {
        if free.len() == wanted {
            break;
        }
        if keymap.layouts_for_key(keycode)? > 0 {
            continue;
        }
        if let Some(name) = keymap.key_name(keycode)? {
            free.push((keycode, name, true));
            continue;
        }
        if let Some(name) = unused_name(keymap, keycode)? {
            free.push((keycode, name, false));
        }
    }
    Ok(free)
}

fn unused_name(keymap: &XkbKeymap, keycode: u32) -> Result<Option<String>, AdapterError> {
    for prefix in NAME_PREFIXES {
        let name = format!("{prefix}{keycode}");
        if !keymap.has_key_name(&name)? {
            return Ok(Some(name));
        }
    }
    Ok(None)
}

fn verified_table(
    text: &str,
    placements: &[KeyPlacement],
    layout: u32,
) -> Result<KeyTable, AdapterError> {
    let rejected = || {
        keymap_refused(
            "The extended keyboard keymap did not place every requested key",
            "extended_keymap_invalid",
            json!({}),
        )
    };
    let compiled = XkbKeymap::compile(text.as_bytes())?.ok_or_else(rejected)?;
    let table = KeyTable::from_compiled(&compiled, layout)?;
    let placed = placements.iter().all(|placement| {
        let expected = EvdevKey {
            code: placement.xkb_keycode - EVDEV_OFFSET,
            shift: placement.level == 2,
        };
        table.lookup(placement.keysym) == Some(expected)
    });
    if !placed {
        return Err(rejected());
    }
    Ok(table)
}

#[cfg(test)]
#[path = "owner_keymap_tests.rs"]
mod tests;
