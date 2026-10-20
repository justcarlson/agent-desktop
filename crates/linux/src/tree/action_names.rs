use agent_desktop_core::{Action, LocatorField, capability};

use super::RawNode;
use super::capability_rules::{Facts, rule_tokens, scroll_tokens};
use super::role_numbers::is_menu_item;

const TABLE: [(&str, &[&str]); 16] = [
    ("click", &[capability::CLICK]),
    ("press", &[capability::CLICK]),
    ("activate", &[capability::CLICK]),
    ("open", &[capability::CLICK]),
    ("jump", &[capability::CLICK]),
    ("clickancestor", &[capability::CLICK]),
    ("select", &[capability::SELECT]),
    ("check", &[capability::TOGGLE, capability::CLICK]),
    ("uncheck", &[capability::TOGGLE]),
    ("toggle", &[capability::TOGGLE]),
    ("switch", &[capability::TOGGLE]),
    ("showcontextmenu", &[capability::RIGHT_CLICK]),
    ("menu", &[capability::RIGHT_CLICK]),
    ("showmenu", &[capability::RIGHT_CLICK]),
    ("menu.popup", &[capability::RIGHT_CLICK]),
    ("setfocus", &[capability::SET_FOCUS]),
];

/// The core capabilities an advertised action name stands for. Names match
/// case-insensitively. A GAction name such as `clipboard.copy` or
/// `window.close`, and `doDefault`, stand for nothing: the first is not an
/// affordance of the node itself, and Chromium advertises `doDefault` on
/// every node, static text included, so it says nothing about whether the
/// node can be acted on. `check` stands for `Click` as well as `Toggle`
/// because Chromium's checkbox runs its page handler through it, and
/// `clickAncestor` stands for `Click` because the ancestor that owns the
/// handler is reached through it.
pub(crate) fn capability_of(name: &str) -> &'static [&'static str] {
    let lowered = name.to_ascii_lowercase();
    TABLE
        .iter()
        .find(|(known, _)| *known == lowered)
        .map_or(&[], |(_, tokens)| *tokens)
}

fn push_new(tokens: &mut Vec<String>, token: &str) {
    if !tokens.iter().any(|known| known == token) {
        tokens.push(token.to_owned());
    }
}

/// The capability tokens of a node, in order of first appearance: its action
/// names through the one table, then the tokens that need more than a name.
/// The result is unknown when a read the tokens depend on failed: a list that
/// might be missing a token must not read as a complete one.
pub(crate) fn capabilities(node: &RawNode) -> LocatorField<Vec<String>> {
    let Some(facts) = Facts::of(node) else {
        return LocatorField::Unknown;
    };
    let mut tokens: Vec<String> = Vec::new();
    for name in facts.names {
        for token in capability_of(name) {
            push_new(&mut tokens, token);
        }
    }
    for token in rule_tokens(&facts) {
        push_new(&mut tokens, token);
    }
    for token in scroll_tokens(node) {
        push_new(&mut tokens, token);
    }
    LocatorField::Known(tokens)
}

/// The action names a click tries, in order. A menu item whose only verb is
/// Qt's `ShowMenu` takes that verb last; it is the menu item's own open action,
/// not the context menu a `showContextMenu` advertises elsewhere.
pub(crate) fn click_verbs(role: Option<u32>) -> &'static [&'static str] {
    const CLICK: [&str; 7] = [
        "click",
        "press",
        "activate",
        "open",
        "jump",
        "check",
        "clickAncestor",
    ];
    const MENU_ITEM_CLICK: [&str; 8] = [
        "click",
        "press",
        "activate",
        "open",
        "jump",
        "check",
        "clickAncestor",
        "ShowMenu",
    ];
    if role.is_some_and(is_menu_item) {
        &MENU_ITEM_CLICK
    } else {
        &CLICK
    }
}

/// The action names a dispatch tries for `action`, in order.
pub(crate) fn verbs_for(action: &Action) -> &'static [&'static str] {
    match action {
        Action::Click => &[
            "click",
            "press",
            "activate",
            "open",
            "jump",
            "check",
            "clickAncestor",
        ],
        Action::RightClick => &["showContextMenu", "menu", "ShowMenu", "menu.popup"],
        Action::Toggle => toggle_verbs(None),
        Action::Check => toggle_verbs(Some(true)),
        Action::Uncheck => toggle_verbs(Some(false)),
        Action::SetFocus => &["setfocus"],
        Action::Expand => &[
            "listitem.toggle-expand",
            "listitem.expand",
            "click",
            "press",
            "activate",
        ],
        Action::Collapse => &[
            "listitem.toggle-expand",
            "listitem.collapse",
            "click",
            "press",
            "activate",
        ],
        Action::Select(_) => &["select"],
        _ => &[],
    }
}

/// The action names a toggle tries when the wanted state is `desired`: the
/// verb that fixes the state first, then the verbs that flip it.
pub(crate) fn toggle_verbs(desired: Option<bool>) -> &'static [&'static str] {
    match desired {
        Some(true) => &["check", "toggle", "switch", "click"],
        Some(false) => &["uncheck", "toggle", "switch", "click", "check"],
        None => &["toggle", "switch", "click", "check", "uncheck"],
    }
}

#[cfg(test)]
#[path = "action_names_tests.rs"]
mod tests;
