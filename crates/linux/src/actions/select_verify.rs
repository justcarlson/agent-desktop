use super::context::Target;
use super::select::handle_of;
use super::select_exclusive::child_selected;
use super::select_search::name_matches;
use crate::system::SharedBus;
use crate::tree::AtspiElement;
use crate::tree::action_read::{NodeView, read_node_view};
use crate::tree::live_offscreen::toolkit_of;
use crate::tree::role_numbers::COMBO_BOX;
use crate::tree::state_bits::SELECTED;
use crate::tree::toolkit_trust::{combo_name_is_value, combo_selection_is_live};

const OBJECT_REPLACEMENT: char = '\u{fffc}';

pub(super) fn verification(combo_value: Option<bool>, item_selected: Option<bool>) -> Option<bool> {
    combo_value.or(item_selected)
}

pub(super) fn says_something(shown: &str) -> bool {
    shown
        .chars()
        .any(|char| !char.is_whitespace() && char != OBJECT_REPLACEMENT)
}

pub(super) fn combo_evidence(
    value: Option<&str>,
    name: Option<&str>,
    wanted: &str,
    name_is_value: bool,
) -> Option<bool> {
    if let Some(shown) = value.filter(|shown| says_something(shown)) {
        return Some(name_matches(shown, wanted));
    }
    let name = name.filter(|name| says_something(name))?;
    match (name_matches(name, wanted), name_is_value) {
        (true, _) => Some(true),
        (false, true) => Some(false),
        (false, false) => None,
    }
}

fn combo_value_matches(view: &NodeView, wanted: &str, name_is_value: bool) -> Option<bool> {
    if view.role() != Some(COMBO_BOX) {
        return None;
    }
    combo_evidence(view.value(), view.name(), wanted, name_is_value)
}

fn holder_selection_is_live(holder: &Target<'_>) -> bool {
    holder
        .view()
        .is_ok_and(|view| view.role() == Some(COMBO_BOX))
        && combo_selection_is_live(&toolkit_of(holder.bus, holder.element, holder.deadline))
}

pub(super) fn read_verification(
    bus: &SharedBus,
    root: &Target<'_>,
    item: &AtspiElement,
    wanted: &str,
    container: Option<(&Target<'_>, i32)>,
) -> Option<bool> {
    let name_is_value = combo_name_is_value(&toolkit_of(bus, root.element, root.deadline));
    let combo = read_node_view(bus, root.handle, root.deadline)
        .ok()
        .and_then(|view| combo_value_matches(&view, wanted, name_is_value));
    let selected = read_node_view(bus, &handle_of(item), root.deadline)
        .ok()
        .and_then(|view| view.state(SELECTED));
    let child = container
        .filter(|(holder, _)| holder_selection_is_live(holder))
        .and_then(|(holder, index)| child_selected(holder, index));
    verification(combo, child.or(selected))
}
