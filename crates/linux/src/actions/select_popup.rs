use agent_desktop_core::{
    Action, ActionResult, AdapterError, DeliverySemantics, ErrorCode, NativeHandle,
};
use serde_json::json;

use super::action_call::resolve_verb;
use super::chain::{DeliveryOutcome, Observed};
use super::click::{
    POLL_STEP, deliver, invoke_verb, names_of, opens_popup, poll_until, popup_inner, refusal,
    run_single_rung,
};
use super::context::Target;
use super::disclosure::set_expanded;
use super::select::{
    BusReader, SELECT_SETTLE, advertises_selection, children_of, container_target, handle_of,
    miss_error, no_route, parent_of, unchanged_is_ignored,
};
use super::select_exclusive::selected_child;
use super::select_identity::with_current_row;
use super::select_search::{Search, name_matches, search};
use crate::tree::AtspiElement;
use crate::tree::action_names::verbs_for;
use crate::tree::action_read::read_node_view;
use crate::tree::role_numbers::{COMBO_BOX, LABEL};
use crate::tree::state_bits::EXPANDED;

const COMMIT_VERBS: [&str; 1] = ["default.activate"];
const POPOVER_CLIMB: usize = 6;
const LABEL_SCAN_LIMIT: usize = 64;
const LABEL_DEPTH_LIMIT: usize = 8;

pub(super) fn is_popup_dropdown(target: &Target<'_>) -> Result<bool, AdapterError> {
    let view = target.view()?;
    if view.role() != Some(COMBO_BOX) || !opens_popup(&view) || advertises_selection(&view) {
        return Ok(false);
    }
    Ok(popup_inner(target, verbs_for(&Action::Expand))?.is_some())
}

fn first_label(reader: &mut BusReader<'_>, root: &AtspiElement) -> Option<String> {
    let mut stack = vec![(root.clone(), 0usize)];
    let mut seen = 0;
    while let Some((node, depth)) = stack.pop() {
        seen += 1;
        if seen > LABEL_SCAN_LIMIT {
            return None;
        }
        let view = read_node_view(reader.bus, &handle_of(&node), reader.deadline).ok()?;
        if view.role() == Some(LABEL) {
            return view.name().map(str::to_owned);
        }
        if depth < LABEL_DEPTH_LIMIT {
            let children = children_of(reader.bus, &node, view.role(), reader.deadline).ok()?;
            stack.extend(children.into_iter().rev().map(|child| (child, depth + 1)));
        }
    }
    None
}

pub(super) fn displayed_value(target: &Target<'_>) -> Option<String> {
    let inner = popup_inner(target, verbs_for(&Action::Expand)).ok()??;
    let inner_target = inner.target(target).ok()?;
    let mut reader = BusReader {
        bus: target.bus,
        deadline: target.deadline,
        inspected_role: None,
    };
    first_label(&mut reader, inner_target.element)
}

fn popover_of(
    reader: &mut BusReader<'_>,
    target: &Target<'_>,
    list: &AtspiElement,
) -> Result<Option<NativeHandle>, AdapterError> {
    let mut node = list.clone();
    for _ in 0..POPOVER_CLIMB {
        let Some(parent) = parent_of(reader.bus, &node, reader.deadline)? else {
            return Ok(None);
        };
        if parent.bus_name() == target.element.bus_name() && parent.path() == target.element.path()
        {
            return Ok(None);
        }
        let handle = handle_of(&parent);
        let view = read_node_view(reader.bus, &handle, reader.deadline)?;
        if view
            .action_names()
            .is_some_and(|names| resolve_verb(names, &COMMIT_VERBS).is_some())
        {
            return Ok(Some(handle));
        }
        node = parent;
    }
    Ok(None)
}

fn choose(target: &Target<'_>, display_value: &str) -> Result<ActionResult, AdapterError> {
    let mut reader = BusReader {
        bus: target.bus,
        deadline: target.deadline,
        inspected_role: None,
    };
    let hit = match search(&mut reader, target.element, display_value)? {
        Search::Found(hit) => hit,
        miss => return Err(miss_error(&miss)),
    };
    let container = hit.container.ok_or_else(no_route)?;
    let list_handle = handle_of(&container.node);
    let list = container_target(target, &list_handle)?;
    let popover_handle = popover_of(&mut reader, target, &container.node)?.ok_or_else(|| {
        refusal(
            ErrorCode::ActionNotSupported,
            "The open popup has no default action that commits a choice",
            json!({ "selection_route": "no_commit" }),
        )
    })?;
    let popover = container_target(target, &popover_handle)?;
    let popover_view = popover.view()?;
    let names = names_of(&popover_view)?;
    let action = Action::Select(display_value.to_owned());
    let row = hit.row.as_ref().unwrap_or(&hit.item);
    let run = || {
        let chosen = super::select_index::write(&list, row)?;
        if chosen == super::mutation::WriteReply::Absent {
            return Ok(DeliveryOutcome::NotDelivered);
        }
        let selected = selected_child(&list).ok_or_else(|| {
            AdapterError::new(ErrorCode::StaleRef, "The selected row is unreadable")
        });
        let committed = with_current_row(selected, row, || {
            invoke_verb(&popover, &action, names, &COMMIT_VERBS)
        })
        .map_err(|error| error.with_disposition(DeliverySemantics::unknown()));
        if matches!(committed, Ok(super::mutation::WriteReply::Absent)) {
            return Ok(DeliveryOutcome::Delivered(Observed::Unread));
        }
        deliver(committed, || {
            poll_until(
                target.deadline,
                SELECT_SETTLE,
                POLL_STEP,
                || displayed_value(target).map(|shown| name_matches(&shown, display_value)),
                |matched| *matched == Some(true),
            )
        })
        .and_then(unchanged_is_ignored)
    };
    run_single_rung(action.name(), target.deadline, run)
}

pub(super) fn select_in_popup(
    target: &Target<'_>,
    display_value: &str,
) -> Result<ActionResult, AdapterError> {
    set_expanded(target, true)?;
    let outcome = choose(target, display_value);
    let still_open = target
        .view()
        .ok()
        .and_then(|view| view.state(EXPANDED))
        .unwrap_or(false);
    if still_open {
        let _ = set_expanded(target, false);
    }
    outcome
}
