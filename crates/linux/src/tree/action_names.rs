use agent_desktop_core::{Action, LocatorField, capability};

use super::capability_rules::scroll_tokens;
use super::role_numbers::{ENTRY, PASSWORD_TEXT, TERMINAL, TEXT};
use super::state_bits::{EDITABLE, StateBits};
use super::{Field, RawNode};

const TABLE: [(&str, &str); 15] = [
    ("click", capability::CLICK),
    ("press", capability::CLICK),
    ("activate", capability::CLICK),
    ("open", capability::CLICK),
    ("jump", capability::CLICK),
    ("select", capability::SELECT),
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
/// match case-insensitively. A delegating name such as `clickAncestor`, a
/// GAction name such as `clipboard.copy` or `window.close`, and `doDefault`
/// stand for nothing: the first two are not an affordance of the node itself,
/// and Chromium advertises `doDefault` on every node, static text included,
/// so it says nothing about whether the node can be acted on.
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
fn capabilities_from(
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

/// The capability tokens of a node: its action names, role and states read
/// through the one table, then the tokens that need more than a name.
pub(crate) fn capabilities(node: &RawNode) -> LocatorField<Vec<String>> {
    let LocatorField::Known(mut tokens) =
        capabilities_from(&node.actions, &node.traits.role, &node.traits.states)
    else {
        return LocatorField::Unknown;
    };
    for token in scroll_tokens(node) {
        if !tokens.iter().any(|known| known == token) {
            tokens.push(token.to_owned());
        }
    }
    LocatorField::Known(tokens)
}

/// The action names a dispatch tries for `action`, in order.
#[expect(dead_code, reason = "no caller yet")]
pub(crate) fn verbs_for(_action: &Action) -> &'static [&'static str] {
    &[]
}

/// The action names a toggle tries when the wanted state is `desired`.
#[expect(dead_code, reason = "no caller yet")]
pub(crate) fn toggle_verbs(_desired: Option<bool>) -> &'static [&'static str] {
    &[]
}

#[cfg(test)]
#[path = "action_names_tests.rs"]
mod tests;
