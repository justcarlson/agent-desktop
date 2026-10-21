use agent_desktop_core::AdapterError;
use zbus::zvariant::{OwnedObjectPath, OwnedValue};

use super::context::Target;
use super::mutation::{WriteKind, WriteReply};
use super::select_identity::with_current_row;
use super::write_call::{Call, invoke};
use crate::system::{ACCESSIBLE_INTERFACE, PROPERTIES_INTERFACE, within};
use crate::tree::AtspiElement;

pub(super) const SELECTION_INTERFACE: &str = "org.a11y.atspi.Selection";

pub(super) fn selected_count(holder: &Target<'_>) -> Option<i32> {
    let connection = holder.bus.get(holder.deadline).ok()?;
    let element = holder.element;
    within(holder.deadline, async {
        let reply = connection
            .inner()
            .call_method(
                Some(element.bus_name()),
                element.path(),
                Some(PROPERTIES_INTERFACE),
                "Get",
                &(SELECTION_INTERFACE, "NSelectedChildren"),
            )
            .await;
        Ok(reply.ok().and_then(|message| {
            let value = message.body().deserialize::<OwnedValue>().ok()?;
            i32::try_from(value).ok()
        }))
    })
    .ok()
    .flatten()
}

fn read_child(
    holder: &Target<'_>,
    interface: &str,
    method: &str,
    index: i32,
) -> Result<AtspiElement, AdapterError> {
    let connection = holder.bus.get(holder.deadline)?;
    let element = holder.element;
    within(holder.deadline, async {
        let reply = connection
            .inner()
            .call_method(
                Some(element.bus_name()),
                element.path(),
                Some(interface),
                method,
                &(index,),
            )
            .await
            .map_err(|error| AdapterError::internal(error.to_string()))?;
        let (name, path) = reply
            .body()
            .deserialize::<(String, OwnedObjectPath)>()
            .map_err(|error| AdapterError::internal(error.to_string()))?;
        Ok(AtspiElement::new(name, path.to_string(), element.pid()))
    })
}

pub(super) fn selected_child(holder: &Target<'_>) -> Option<AtspiElement> {
    if selected_count(holder) != Some(1) {
        return None;
    }
    read_child(holder, SELECTION_INTERFACE, "GetSelectedChild", 0).ok()
}

pub(super) fn select_child(
    holder: &Target<'_>,
    index: i32,
    row: &AtspiElement,
) -> Result<WriteReply, AdapterError> {
    with_current_row(
        read_child(holder, ACCESSIBLE_INTERFACE, "GetChildAtIndex", index),
        row,
        || {
            invoke(
                holder,
                Call {
                    interface: SELECTION_INTERFACE,
                    method: "SelectChild",
                    body: &(index,),
                    kind: WriteKind::SelectChild,
                },
            )
        },
    )
}

pub(super) fn needs_replacement(count: Option<i32>) -> bool {
    count.is_some_and(|count| count > 1)
}

pub(super) fn exclusive_verdict(count: Option<i32>) -> Option<bool> {
    match count {
        Some(1) => Some(true),
        _ => None,
    }
}

pub(super) fn replace_with_only(
    holder: &Target<'_>,
    index: i32,
    row: &AtspiElement,
) -> Result<bool, AdapterError> {
    let cleared = with_current_row(
        read_child(holder, ACCESSIBLE_INTERFACE, "GetChildAtIndex", index),
        row,
        || {
            invoke(
                holder,
                Call {
                    interface: SELECTION_INTERFACE,
                    method: "ClearSelection",
                    body: &(),
                    kind: WriteKind::ClearSelection,
                },
            )
        },
    )? == WriteReply::Done;
    if !cleared {
        return Ok(false);
    }
    Ok(select_child(holder, index, row)? == WriteReply::Done)
}

#[cfg(test)]
#[path = "select_exclusive_tests.rs"]
mod tests;
