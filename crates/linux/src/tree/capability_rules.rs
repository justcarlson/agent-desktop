use agent_desktop_core::capability;

use super::role_numbers::{
    CHECK_BOX, CHECK_MENU_ITEM, COMBO_BOX, DIAL, ENTRY, LIST, PASSWORD_TEXT, PUSH_BUTTON,
    RADIO_BUTTON, RADIO_MENU_ITEM, SCROLL_BAR, SLIDER, SPIN_BUTTON, TERMINAL, TEXT, TOGGLE_BUTTON,
};
use super::state_bits::{EDITABLE, EXPANDABLE, EXPANDED, HAS_POPUP, READ_ONLY, StateBits};
use super::{Field, RawNode};

const VALUE: &str = "org.a11y.atspi.Value";
const TEXT_INTERFACE: &str = "org.a11y.atspi.Text";
const EDITABLE_TEXT: &str = "org.a11y.atspi.EditableText";
const SELECTION: &str = "org.a11y.atspi.Selection";
const TOGGLE_EXPAND: &str = "listitem.toggle-expand";
const EXPAND: &str = "listitem.expand";
const COLLAPSE: &str = "listitem.collapse";
const DISCLOSURE_CLICKS: [&str; 3] = ["click", "press", "activate"];
const TOGGLE_ONLY_NAMES: [&str; 3] = ["toggle", "switch", "uncheck"];

/// What the token rules read from a node, every part confirmed. A part whose
/// read failed makes the whole node unknown, never empty: a token the failed
/// read would have decided must not silently be left out.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Facts<'a> {
    pub(crate) role: u32,
    pub(crate) bits: StateBits<'a>,
    pub(crate) interfaces: &'a [String],
    pub(crate) names: &'a [String],
    pub(crate) child_count: Option<i32>,
}

fn interface_decides(role: u32) -> bool {
    is_text_entry(role) || is_numeric_control(role) || matches!(role, LIST | COMBO_BOX)
}

fn confirmed(field: &Field<Vec<String>>) -> Option<&[String]> {
    match field {
        Field::Present(list) => Some(list),
        Field::Absent => Some(&[]),
        Field::Failed(_) => None,
    }
}

impl<'a> Facts<'a> {
    /// The facts of a node, or `None` when the role, the states, the action
    /// names, the interfaces of a role they decide (text, numeric, list and
    /// combo box) or, on a node that advertises `Selection`, the child count
    /// could not be read. A layout node whose interface list failed keeps the
    /// tokens its names give: no interface could add one it would claim.
    pub(crate) fn of(node: &'a RawNode) -> Option<Self> {
        let role = *node.traits.role.present()?;
        let words = node.traits.states.present()?;
        let interfaces = match &node.traits.interfaces {
            Field::Failed(_) if !interface_decides(role) => &[],
            other => confirmed(other)?,
        };
        let names = confirmed(&node.actions)?;
        let child_count = node.properties.child_count.present().copied();
        let facts = Self {
            role,
            bits: StateBits::new(words),
            interfaces,
            names,
            child_count,
        };
        if facts.advertises(SELECTION) && node.properties.child_count.is_failed() {
            return None;
        }
        Some(facts)
    }

    fn advertises(&self, interface: &str) -> bool {
        self.interfaces.iter().any(|name| name == interface)
    }

    fn has_name(&self, wanted: &str) -> bool {
        self.names
            .iter()
            .any(|name| name.eq_ignore_ascii_case(wanted))
    }

    fn has_any_name(&self, wanted: &[&str]) -> bool {
        wanted.iter().any(|name| self.has_name(name))
    }
}

fn is_text_entry(role: u32) -> bool {
    matches!(role, TEXT | TERMINAL | ENTRY | PASSWORD_TEXT)
}

fn is_numeric_control(role: u32) -> bool {
    matches!(role, SLIDER | SPIN_BUTTON | DIAL | SCROLL_BAR)
}

fn text_tokens(facts: &Facts) -> Vec<&'static str> {
    let editable = is_text_entry(facts.role)
        && facts.bits.has(EDITABLE)
        && !facts.bits.has(READ_ONLY)
        && facts.advertises(EDITABLE_TEXT);
    let mut tokens = Vec::new();
    if editable {
        tokens.push(capability::SET_VALUE);
        if facts.advertises(TEXT_INTERFACE) {
            tokens.push(capability::TYPE_TEXT);
        }
    }
    tokens
}

fn value_tokens(facts: &Facts) -> Vec<&'static str> {
    if is_numeric_control(facts.role) && facts.advertises(VALUE) {
        vec![capability::SET_VALUE]
    } else {
        Vec::new()
    }
}

fn popup_button_children(facts: &Facts) -> bool {
    match facts.child_count {
        Some(1) => true,
        Some(2) => facts.bits.has(EXPANDED),
        _ => false,
    }
}

fn menu_button(facts: &Facts) -> bool {
    facts.role == PUSH_BUTTON
        && facts.bits.has(HAS_POPUP)
        && popup_button_children(facts)
        && !facts.has_any_name(&DISCLOSURE_CLICKS)
}

fn disclosure_tokens(facts: &Facts) -> Vec<&'static str> {
    let stateful = facts.bits.has(EXPANDABLE) || facts.bits.has(EXPANDED);
    let clickable = facts.has_any_name(&DISCLOSURE_CLICKS);
    let both = (stateful && (clickable || facts.has_name(TOGGLE_EXPAND))) || menu_button(facts);
    let expand = both || (stateful && facts.has_name(EXPAND));
    let collapse = both || (stateful && facts.has_name(COLLAPSE));
    let mut tokens = Vec::new();
    if expand {
        tokens.push(capability::EXPAND);
    }
    if collapse {
        tokens.push(capability::COLLAPSE);
    }
    tokens
}

pub(crate) fn carries_checked(role: u32) -> bool {
    matches!(
        role,
        CHECK_BOX | RADIO_BUTTON | CHECK_MENU_ITEM | RADIO_MENU_ITEM | TOGGLE_BUTTON
    )
}

fn toggle_click_tokens(facts: &Facts) -> Vec<&'static str> {
    if carries_checked(facts.role) && facts.has_any_name(&TOGGLE_ONLY_NAMES) {
        vec![capability::CLICK]
    } else {
        Vec::new()
    }
}

fn select_tokens(facts: &Facts) -> Vec<&'static str> {
    if facts.advertises(SELECTION) && facts.child_count.is_some_and(|count| count > 0) {
        vec![capability::SELECT]
    } else {
        Vec::new()
    }
}

/// The tokens that need more than an action name: the text and numeric
/// writes, the disclosure pair, selection and the click of a switch that
/// advertises only a toggle verb. A scroll bar takes `SetValue`
/// on every toolkit and the write is judged by its read-back, because the
/// table has no toolkit input and a Qt bar that does not move content says
/// so when the write is read back.
pub(crate) fn rule_tokens(facts: &Facts) -> Vec<&'static str> {
    let mut tokens = text_tokens(facts);
    tokens.extend(value_tokens(facts));
    tokens.extend(disclosure_tokens(facts));
    tokens.extend(select_tokens(facts));
    tokens.extend(toggle_click_tokens(facts));
    tokens
}

/// The tokens a scroll target carries. A scroll target has no token yet.
pub(crate) fn scroll_tokens(_node: &RawNode) -> Vec<&'static str> {
    Vec::new()
}

#[cfg(test)]
#[path = "capability_rules_tests.rs"]
mod tests;
