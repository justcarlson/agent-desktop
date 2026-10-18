use agent_desktop_core::{LocatorField, capability};

use super::Field;
use super::role_numbers::{ENTRY, PASSWORD_TEXT, TERMINAL, TEXT};
use super::state_bits::{EDITABLE, StateBits};

const TABLE: [(&str, &str); 15] = [
    ("click", capability::CLICK),
    ("press", capability::CLICK),
    ("activate", capability::CLICK),
    ("dodefault", capability::CLICK),
    ("open", capability::CLICK),
    ("jump", capability::CLICK),
    ("check", capability::TOGGLE),
    ("uncheck", capability::TOGGLE),
    ("toggle", capability::TOGGLE),
    ("switch", capability::TOGGLE),
    ("showcontextmenu", capability::RIGHT_CLICK),
    ("menu", capability::RIGHT_CLICK),
    ("showmenu", capability::RIGHT_CLICK),
    ("menu.popup", capability::RIGHT_CLICK),
    ("setfocus", capability::SET_FOCUS),
];

fn is_text_entry(role: u32) -> bool {
    matches!(role, TEXT | TERMINAL | ENTRY | PASSWORD_TEXT)
}

/// The core capability an advertised action name stands for, if any. Names
/// match case-insensitively. A delegating name such as `clickAncestor` and a
/// GAction name such as `clipboard.copy` or `window.close` stand for nothing:
/// they are not an affordance of the node itself.
pub(crate) fn capability_of(name: &str) -> Option<&'static str> {
    let lowered = name.to_ascii_lowercase();
    TABLE
        .iter()
        .find(|(known, _)| *known == lowered)
        .map(|(_, token)| *token)
}

/// The capability tokens of a node, in order of first appearance. The
/// result is unknown when the action list, or the role and state reads that
/// decide `SetValue`, failed: a list that might be missing a token must not
/// read as a complete one. `SetValue` is a positive claim from the
/// `editable` state on a text role.
pub(crate) fn capabilities(
    actions: &Field<Vec<String>>,
    role: &Field<u32>,
    states: &Field<Vec<u32>>,
) -> LocatorField<Vec<String>> {
    let (Some(role), Some(states)) = (role.present(), states.present()) else {
        return LocatorField::Unknown;
    };
    let names: &[String] = match actions {
        Field::Present(names) => names,
        Field::Absent => &[],
        Field::Failed(_) => return LocatorField::Unknown,
    };
    let mut tokens: Vec<String> = Vec::new();
    let mut push = |token: &str| {
        if !tokens.iter().any(|known| known == token) {
            tokens.push(token.to_owned());
        }
    };
    names
        .iter()
        .filter_map(|name| capability_of(name))
        .for_each(&mut push);
    if is_text_entry(*role) && StateBits::new(states).has(EDITABLE) {
        push(capability::SET_VALUE);
    }
    LocatorField::Known(tokens)
}

#[cfg(test)]
#[path = "action_names_tests.rs"]
mod tests;
