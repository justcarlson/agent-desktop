use agent_desktop_core::AdapterError;
use zbus::export::serde::Serialize;
use zbus::zvariant::{DynamicType, OwnedValue};

use super::context::Target;
use super::mutation::{WriteKind, WriteReply};
use super::write_call::{Call, invoke};
use crate::system::{PROPERTIES_INTERFACE, within};

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

pub(super) fn child_selected(holder: &Target<'_>, index: i32) -> Option<bool> {
    let connection = holder.bus.get(holder.deadline).ok()?;
    let element = holder.element;
    within(holder.deadline, async {
        let reply = connection
            .inner()
            .call_method(
                Some(element.bus_name()),
                element.path(),
                Some(SELECTION_INTERFACE),
                "IsChildSelected",
                &(index,),
            )
            .await;
        Ok(reply
            .ok()
            .and_then(|message| message.body().deserialize::<bool>().ok()))
    })
    .ok()
    .flatten()
}

pub(super) fn needs_replacement(count: Option<i32>) -> bool {
    count.is_some_and(|count| count > 1)
}

pub(super) fn exclusive_verdict(count: Option<i32>) -> Option<bool> {
    match count {
        Some(1) | None => Some(true),
        Some(_) => None,
    }
}

fn write(
    holder: &Target<'_>,
    call: Call<'_, impl Serialize + DynamicType>,
) -> Result<bool, AdapterError> {
    Ok(invoke(holder, call, true)? == WriteReply::Done)
}

pub(super) fn replace_with_only(holder: &Target<'_>, index: i32) -> Result<bool, AdapterError> {
    let cleared = write(
        holder,
        Call {
            interface: SELECTION_INTERFACE,
            method: "ClearSelection",
            body: &(),
            kind: WriteKind::ClearSelection,
        },
    )?;
    if !cleared {
        return Ok(false);
    }
    write(
        holder,
        Call {
            interface: SELECTION_INTERFACE,
            method: "SelectChild",
            body: &(index,),
            kind: WriteKind::SelectChild,
        },
    )
}

#[cfg(test)]
#[path = "select_exclusive_tests.rs"]
mod tests;
