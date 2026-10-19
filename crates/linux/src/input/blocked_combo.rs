use agent_desktop_core::{KeyCombo, Modifier};

use super::key_names::keysym_for_key;

const FIRST_VIRTUAL_TERMINAL_KEYSYM: u32 = 0xffbe;
const LAST_VIRTUAL_TERMINAL_KEYSYM: u32 = 0xffc9;
const LOWERCASE_OFFSET: u32 = 0x20;

const BLOCKED: &[(&[Modifier], &str)] = &[
    (&[Modifier::Alt], "f4"),
    (&[Modifier::Alt], "tab"),
    (&[Modifier::Meta], "l"),
    (&[Modifier::Meta], "d"),
    (&[Modifier::Meta], "tab"),
    (&[Modifier::Alt], "f2"),
    (&[Modifier::Ctrl, Modifier::Alt], "t"),
    (&[Modifier::Ctrl, Modifier::Alt], "delete"),
    (&[Modifier::Ctrl, Modifier::Alt], "forwarddelete"),
    (&[Modifier::Ctrl, Modifier::Alt], "backspace"),
    (&[Modifier::Ctrl, Modifier::Alt], "l"),
    (&[], "super"),
];

pub(crate) fn is_blocked(combo: &KeyCombo) -> bool {
    let Ok(key) = keysym_for_key(&combo.key).map(fold_case) else {
        return false;
    };
    let held = |needed: &[Modifier]| needed.iter().all(|m| combo.modifiers.contains(m));
    let listed = BLOCKED.iter().any(|(needed, name)| {
        held(needed) && keysym_for_key(name).is_ok_and(|listed| fold_case(listed) == key)
    });
    listed
        || (held(&[Modifier::Ctrl, Modifier::Alt])
            && (FIRST_VIRTUAL_TERMINAL_KEYSYM..=LAST_VIRTUAL_TERMINAL_KEYSYM).contains(&key))
}

fn fold_case(keysym: u32) -> u32 {
    if (0x41..=0x5a).contains(&keysym) {
        keysym + LOWERCASE_OFFSET
    } else {
        keysym
    }
}

#[cfg(test)]
#[path = "blocked_combo_tests.rs"]
mod tests;
