use std::time::Duration;

use agent_desktop_core::{
    Action, ActionResult, AdapterError, Deadline, DeliverySemantics, ErrorCode,
};
use serde_json::json;
use zbus::zvariant::{OwnedValue, Structure, Value};

use super::action_call::resolve_verb;
use super::chain::{DeliveryOutcome, Observed};
use super::click::{
    POLL_STEP, deliver, invoke_verb, names_of, poll_until, refusal, run_single_rung,
};
use super::context::Target;
use super::mutation::{WriteKind, WriteReply};
use super::select_search::{Container, Hit, Inspected, ItemReader, Search, name_matches, search};
use super::write_call::{Call, invoke};
use crate::system::{ACCESSIBLE_INTERFACE, PROPERTIES_INTERFACE, SharedBus, within};
use crate::tree::action_names::verbs_for;
use crate::tree::action_read::{NodeView, read_node_view};
use crate::tree::role_numbers::COMBO_BOX;
use crate::tree::state_bits::SELECTED;
use crate::tree::{AtspiElement, ancestry, atspi_element, list_children};

const SELECTION_INTERFACE: &str = "org.a11y.atspi.Selection";
const SELECT_SETTLE: Duration = Duration::from_millis(1000);

pub(super) fn verification(combo_value: Option<bool>, item_selected: Option<bool>) -> Option<bool> {
    combo_value.or(item_selected)
}

struct BusReader<'a> {
    bus: &'a SharedBus,
    deadline: Deadline,
    inspected_role: Option<u32>,
}

fn handle_of(element: &AtspiElement) -> agent_desktop_core::NativeHandle {
    element.clone().into_native_handle()
}

impl ItemReader for BusReader<'_> {
    type Node = AtspiElement;

    fn inspect(&mut self, node: &AtspiElement) -> Result<Inspected, AdapterError> {
        let view = read_node_view(self.bus, &handle_of(node), self.deadline)?;
        self.inspected_role = view.role();
        Ok(Inspected {
            name: view.name().map(str::to_owned),
            selection: advertises_selection(&view),
        })
    }

    fn children(&mut self, node: &AtspiElement) -> Result<Vec<AtspiElement>, AdapterError> {
        children_of(self.bus, node, self.inspected_role, self.deadline)
    }
}

fn advertises_selection(view: &NodeView) -> bool {
    view.interfaces()
        .is_some_and(|names| names.iter().any(|name| name == SELECTION_INTERFACE))
}

fn children_of(
    bus: &SharedBus,
    node: &AtspiElement,
    role: Option<u32>,
    deadline: Deadline,
) -> Result<Vec<AtspiElement>, AdapterError> {
    let connection = bus.get(deadline)?;
    let children = within(deadline, async {
        list_children(connection.inner(), node).await
    })?;
    Ok(children
        .into_iter()
        .map(|(name, path)| {
            let mut child = AtspiElement::new(name, path, node.pid());
            child.set_ancestry(ancestry::child(node, role));
            child
        })
        .collect())
}

fn parent_of(
    bus: &SharedBus,
    node: &AtspiElement,
    deadline: Deadline,
) -> Result<Option<AtspiElement>, AdapterError> {
    let connection = bus.get(deadline)?;
    within(deadline, async {
        let reply = connection
            .inner()
            .call_method(
                Some(node.bus_name()),
                node.path(),
                Some(PROPERTIES_INTERFACE),
                "Get",
                &(ACCESSIBLE_INTERFACE, "Parent"),
            )
            .await;
        let parent = reply.ok().and_then(|message| {
            let value = message.body().deserialize::<OwnedValue>().ok()?;
            let structure = Structure::try_from(value).ok()?;
            match structure.fields() {
                [Value::Str(name), Value::ObjectPath(path)] => {
                    let mut parent =
                        AtspiElement::new(name.to_string(), path.to_string(), node.pid());
                    parent.set_ancestry(node.ancestry().clone());
                    Some(parent)
                }
                _ => None,
            }
        });
        Ok(parent)
    })
}

fn selection_parent(
    reader: &mut BusReader<'_>,
    node: &AtspiElement,
) -> Result<Option<Container<AtspiElement>>, AdapterError> {
    let Some(parent) = parent_of(reader.bus, node, reader.deadline)? else {
        return Ok(None);
    };
    if !reader.inspect(&parent)?.selection {
        return Ok(None);
    }
    let siblings = reader.children(&parent)?;
    let position = siblings
        .iter()
        .position(|sibling| sibling.bus_name() == node.bus_name() && sibling.path() == node.path());
    Ok(position.map(|index| Container {
        node: parent,
        index: i32::try_from(index).unwrap_or(i32::MAX),
    }))
}

fn combo_value_matches(view: &NodeView, wanted: &str) -> Option<bool> {
    if view.role() != Some(COMBO_BOX) {
        return None;
    }
    let shown = view.value().or_else(|| view.name())?;
    Some(name_matches(shown, wanted))
}

fn read_verification(
    bus: &SharedBus,
    root: &Target<'_>,
    item: &AtspiElement,
    wanted: &str,
) -> Option<bool> {
    let combo = read_node_view(bus, root.handle, root.deadline)
        .ok()
        .and_then(|view| combo_value_matches(&view, wanted));
    let selected = read_node_view(bus, &handle_of(item), root.deadline)
        .ok()
        .and_then(|view| view.state(SELECTED));
    verification(combo, selected)
}

fn container_target<'a>(
    parent: &Target<'a>,
    handle: &'a agent_desktop_core::NativeHandle,
) -> Result<Target<'a>, AdapterError> {
    Ok(Target {
        bus: parent.bus,
        handle,
        element: atspi_element(handle)?,
        deadline: parent.deadline,
    })
}

fn select_through(
    target: &Target<'_>,
    hit: &Hit<AtspiElement>,
    container: Option<&Container<AtspiElement>>,
    display_value: &str,
) -> Result<ActionResult, AdapterError> {
    let action = Action::Select(display_value.to_owned());
    let verbs = verbs_for(&action);
    let view = read_node_view(target.bus, &handle_of(&hit.item), target.deadline)?;
    let names = names_of(&view)?;
    let own_verb = hit.own && resolve_verb(names, verbs).is_some();
    let observe = || {
        poll_until(
            target.deadline,
            SELECT_SETTLE,
            POLL_STEP,
            || read_verification(target.bus, target, &hit.item, display_value),
            |verified| *verified == Some(true),
        )
    };
    let run = || {
        let reply: Result<WriteReply, AdapterError> = match (own_verb, container) {
            (true, _) => invoke_verb(target, &action, names, verbs),
            (false, Some(container)) => {
                let handle = handle_of(&container.node);
                let holder = container_target(target, &handle)?;
                invoke(
                    &holder,
                    Call {
                        interface: SELECTION_INTERFACE,
                        method: "SelectChild",
                        body: &(container.index,),
                        kind: WriteKind::SelectChild,
                    },
                    true,
                )
            }
            (false, None) => Err(no_route(display_value)),
        };
        deliver(reply, observe).and_then(unchanged_is_ignored)
    };
    run_single_rung(action.name(), target.deadline, run)
}

pub(super) fn unchanged_is_ignored(
    outcome: DeliveryOutcome,
) -> Result<DeliveryOutcome, AdapterError> {
    match outcome {
        DeliveryOutcome::Delivered(Observed::Unchanged) => Err(AdapterError::new(
            ErrorCode::ActionFailed,
            "The selection still holds the old choice, so the select was not applied",
        )
        .with_suggestion("Choose the item with a headed click and key route")
        .with_details(json!({ "cause": "write_ignored" }))
        .with_disposition(DeliverySemantics::not_delivered())),
        other => Ok(other),
    }
}

fn combo_selection(target: &Target<'_>) -> Result<bool, AdapterError> {
    let view = target.view()?;
    Ok(view.role() == Some(COMBO_BOX) && advertises_selection(&view))
}

fn no_route(display_value: &str) -> AdapterError {
    refusal(
        ErrorCode::ActionNotSupported,
        format!(
            "No semantic route selects the {}-character value; use the headed click and key route",
            display_value.chars().count()
        ),
        json!({ "selection_route": "absent" }),
    )
}

pub(crate) fn select(
    target: &Target<'_>,
    display_value: &str,
) -> Result<ActionResult, AdapterError> {
    let mut reader = BusReader {
        bus: target.bus,
        deadline: target.deadline,
        inspected_role: None,
    };
    match search(&mut reader, target.element, display_value)? {
        Search::Found(hit) => {
            let container = if hit.own {
                let own_names_verb = {
                    let view = target.view()?;
                    resolve_verb(names_of(&view)?, verbs_for(&Action::Select(String::new())))
                        .is_some()
                };
                if own_names_verb {
                    None
                } else {
                    selection_parent(&mut reader, target.element)?
                }
            } else {
                let combo = combo_selection(target)?;
                hit.container.clone().map(|container| match combo {
                    true => Container {
                        node: target.element.clone(),
                        index: container.index,
                    },
                    false => container,
                })
            };
            let hit = Hit {
                own: hit.own && container.is_none(),
                ..hit
            };
            select_through(target, &hit, container.as_ref(), display_value)
        }
        Search::Ambiguous => Err(AdapterError::ambiguous_target(format!(
            "More than one item matches the {}-character value",
            display_value.chars().count()
        ))),
        Search::NotFound { seen } if seen <= 1 => Err(no_route(display_value)),
        Search::NotFound { .. } => Err(refusal(
            ErrorCode::ElementNotFound,
            format!(
                "No item matches the {}-character value",
                display_value.chars().count()
            ),
            json!({ "searched": true }),
        )),
        Search::Incomplete => Err(refusal(
            ErrorCode::ElementNotFound,
            "The search for the item ended at its limit before it covered the container",
            json!({ "search_incomplete": true }),
        )),
    }
}

#[cfg(test)]
#[path = "select_tests.rs"]
mod tests;
