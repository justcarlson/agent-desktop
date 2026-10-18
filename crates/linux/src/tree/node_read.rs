use std::collections::{BTreeMap, HashMap};

use agent_desktop_core::{AdapterError, ErrorCode};
use futures_util::future::{join, join3, join4};
use zbus::export::serde::Serialize;
use zbus::zvariant::{DynamicType, OwnedValue};
use zbus::{Connection, Message};

use super::bounded::collect_bounded;
use super::{AtspiElement, ChildAddress, Extents, Field, NodeProperties, NodeTraits, RawNode};
use crate::system::{
    ACCESSIBLE_INTERFACE, NAME_HAS_NO_OWNER, PROPERTIES_INTERFACE, SERVICE_UNKNOWN,
};

const COMPONENT_INTERFACE: &str = "org.a11y.atspi.Component";
const ACTION_INTERFACE: &str = "org.a11y.atspi.Action";
const UNKNOWN_OBJECT: &str = "org.freedesktop.DBus.Error.UnknownObject";
const UNKNOWN_METHOD: &str = "org.freedesktop.DBus.Error.UnknownMethod";
const ACTION_NAMES_IN_FLIGHT: usize = 4;
pub(super) const NAME: &str = "Name";
const DESCRIPTION: &str = "Description";
const CHILD_COUNT: &str = "ChildCount";
const ACCESSIBLE_ID: &str = "AccessibleId";
const NULL_PATH: &str = "/org/a11y/atspi/null";
const SCREEN_COORDINATES: u32 = 0;

type Reply = Result<Message, zbus::Error>;

pub(super) async fn call<B>(
    connection: &Connection,
    element: &AtspiElement,
    interface: &str,
    method: &str,
    body: &B,
) -> Reply
where
    B: Serialize + DynamicType,
{
    connection
        .call_method(
            Some(element.bus_name()),
            element.path(),
            Some(interface),
            method,
            body,
        )
        .await
}

/// Errors that mean the object or its application no longer exists, as
/// opposed to a read that failed on a node that is still there.
fn gone(error: &zbus::Error, required: bool) -> Option<AdapterError> {
    let zbus::Error::MethodError(name, _, _) = error else {
        return None;
    };
    match name.as_str() {
        SERVICE_UNKNOWN | NAME_HAS_NO_OWNER => Some(
            AdapterError::new(
                ErrorCode::AppNotFound,
                "The application left the accessibility bus",
            )
            .with_platform_detail(error.to_string()),
        ),
        UNKNOWN_OBJECT => Some(element_gone(error)),
        UNKNOWN_METHOD if required => Some(element_gone(error)),
        _ => None,
    }
}

fn element_gone(error: &zbus::Error) -> AdapterError {
    AdapterError::new(
        ErrorCode::ElementNotFound,
        "The accessible object no longer exists",
    )
    .with_platform_detail(error.to_string())
}

fn field<T>(result: Result<T, zbus::Error>, required: bool) -> Result<Field<T>, AdapterError> {
    match result {
        Ok(value) => Ok(Field::Present(value)),
        Err(error) => match gone(&error, required) {
            Some(gone) => Err(gone),
            None => Ok(Field::Failed(error.to_string())),
        },
    }
}

pub(super) fn string_value(value: &OwnedValue) -> Field<String> {
    match value.downcast_ref::<&str>() {
        Ok(text) => Field::Present(text.to_owned()),
        Err(error) => Field::Failed(error.to_string()),
    }
}

fn count_value(value: &OwnedValue) -> Field<i32> {
    match value.downcast_ref::<i32>() {
        Ok(count) => Field::Present(count),
        Err(error) => Field::Failed(error.to_string()),
    }
}

fn property<T>(
    map: &HashMap<String, OwnedValue>,
    key: &str,
    convert: fn(&OwnedValue) -> Field<T>,
) -> Field<T> {
    map.get(key).map_or(Field::Absent, convert)
}

fn from_map(map: &HashMap<String, OwnedValue>) -> NodeProperties {
    NodeProperties {
        name: property(map, NAME, string_value),
        description: property(map, DESCRIPTION, string_value),
        child_count: property(map, CHILD_COUNT, count_value),
        accessible_id: property(map, ACCESSIBLE_ID, string_value),
    }
}

pub(super) async fn get_one<T>(
    connection: &Connection,
    element: &AtspiElement,
    key: &'static str,
    convert: fn(&OwnedValue) -> Field<T>,
) -> Result<Field<T>, AdapterError> {
    let reply = call(
        connection,
        element,
        PROPERTIES_INTERFACE,
        "Get",
        &(ACCESSIBLE_INTERFACE, key),
    )
    .await;
    let read = field(
        reply.and_then(|message| message.body().deserialize::<OwnedValue>()),
        true,
    )?;
    Ok(match read {
        Field::Present(value) => convert(&value),
        Field::Absent => Field::Failed(format!("Properties.Get({key}) failed")),
        Field::Failed(detail) => Field::Failed(format!("Properties.Get({key}) failed: {detail}")),
    })
}

/// Toolkits that do not implement `GetAll` for the accessible interface (Qt
/// answers `UnknownInterface`) still answer each property individually.
async fn properties_one_by_one(
    connection: &Connection,
    element: &AtspiElement,
) -> Result<NodeProperties, AdapterError> {
    let (name, description, child_count, accessible_id) = join4(
        get_one(connection, element, NAME, string_value),
        get_one(connection, element, DESCRIPTION, string_value),
        get_one(connection, element, CHILD_COUNT, count_value),
        get_one(connection, element, ACCESSIBLE_ID, string_value),
    )
    .await;
    Ok(NodeProperties {
        name: name?,
        description: description?,
        child_count: child_count?,
        accessible_id: accessible_id?,
    })
}

async fn read_properties(
    connection: &Connection,
    element: &AtspiElement,
    reply: Reply,
) -> Result<NodeProperties, AdapterError> {
    let map = field(
        reply.and_then(|message| message.body().deserialize::<HashMap<String, OwnedValue>>()),
        false,
    )?;
    match map {
        Field::Present(map) => Ok(from_map(&map)),
        Field::Absent | Field::Failed(_) => properties_one_by_one(connection, element).await,
    }
}

fn read_children(reply: Reply) -> Result<Field<Vec<ChildAddress>>, AdapterError> {
    let raw = field(
        reply.and_then(|message| {
            message
                .body()
                .deserialize::<Vec<(String, zbus::zvariant::OwnedObjectPath)>>()
        }),
        true,
    )?;
    Ok(raw.map(|entries| {
        entries
            .into_iter()
            .filter(|(bus, path)| !bus.is_empty() && path.as_str() != NULL_PATH)
            .map(|(bus, path)| (bus, path.to_string()))
            .collect()
    }))
}

/// `None` when the interface is advertised and its reads should go ahead.
/// When the interface list itself could not be read, nothing is known about
/// the interface, so the dependent field is a failure and not an absence.
pub(super) fn skipped_without<T>(interfaces: &Field<Vec<String>>, name: &str) -> Option<Field<T>> {
    match interfaces {
        Field::Present(list) if list.iter().any(|entry| entry == name) => None,
        Field::Present(_) | Field::Absent => Some(Field::Absent),
        Field::Failed(detail) => Some(Field::Failed(format!(
            "interface list unavailable, {name} not checked: {detail}"
        ))),
    }
}

async fn read_extents(
    connection: &Connection,
    element: &AtspiElement,
    interfaces: &Field<Vec<String>>,
) -> Result<Field<Extents>, AdapterError> {
    if let Some(skipped) = skipped_without(interfaces, COMPONENT_INTERFACE) {
        return Ok(skipped);
    }
    let reply = call(
        connection,
        element,
        COMPONENT_INTERFACE,
        "GetExtents",
        &(SCREEN_COORDINATES,),
    )
    .await;
    field(
        reply.and_then(|message| message.body().deserialize::<(i32, i32, i32, i32)>()),
        false,
    )
    .map(|read| {
        read.map(|(x, y, width, height)| Extents {
            x,
            y,
            width,
            height,
        })
    })
}

async fn action_name(
    connection: &Connection,
    element: &AtspiElement,
    index: i32,
    listed: String,
) -> String {
    let reply = call(connection, element, ACTION_INTERFACE, "GetName", &(index,)).await;
    reply
        .and_then(|message| message.body().deserialize::<String>())
        .ok()
        .filter(|name| !name.is_empty())
        .unwrap_or(listed)
}

/// `GetName(i)` is the only call that returns the machine name on every
/// toolkit: `GetActions` returns empty names on Chromium and junk on Gecko.
/// `GetActions` supplies the count and the fallback for an index whose
/// `GetName` fails.
async fn read_actions(
    connection: &Connection,
    element: &AtspiElement,
    interfaces: &Field<Vec<String>>,
) -> Result<Field<Vec<String>>, AdapterError> {
    if let Some(skipped) = skipped_without(interfaces, ACTION_INTERFACE) {
        return Ok(skipped);
    }
    let reply = call(connection, element, ACTION_INTERFACE, "GetActions", &()).await;
    let listed = field(
        reply.and_then(|message| {
            message
                .body()
                .deserialize::<Vec<(String, String, String)>>()
        }),
        false,
    )?;
    Ok(match listed {
        Field::Present(entries) => Field::Present(
            collect_bounded(
                ACTION_NAMES_IN_FLIGHT,
                entries
                    .into_iter()
                    .enumerate()
                    .map(|(index, (name, _, _))| {
                        action_name(connection, element, index as i32, name)
                    }),
            )
            .await,
        ),
        Field::Absent => Field::Absent,
        Field::Failed(detail) => Field::Failed(detail),
    })
}

/// Reads one node. All fixed calls are in flight together; extents and
/// actions follow only when the interface list names them. An error that
/// means the object or its application is gone fails the read; any other
/// failure stays on the field it hit.
pub(crate) async fn read_node(
    connection: &Connection,
    element: &AtspiElement,
) -> Result<RawNode, AdapterError> {
    let (first, second) = join(
        join4(
            call(
                connection,
                element,
                PROPERTIES_INTERFACE,
                "GetAll",
                &(ACCESSIBLE_INTERFACE,),
            ),
            call(connection, element, ACCESSIBLE_INTERFACE, "GetRole", &()),
            call(
                connection,
                element,
                ACCESSIBLE_INTERFACE,
                "GetRoleName",
                &(),
            ),
            call(connection, element, ACCESSIBLE_INTERFACE, "GetState", &()),
        ),
        join3(
            call(
                connection,
                element,
                ACCESSIBLE_INTERFACE,
                "GetInterfaces",
                &(),
            ),
            call(
                connection,
                element,
                ACCESSIBLE_INTERFACE,
                "GetAttributes",
                &(),
            ),
            call(
                connection,
                element,
                ACCESSIBLE_INTERFACE,
                "GetChildren",
                &(),
            ),
        ),
    )
    .await;
    let (all, role, role_name, state) = first;
    let (interfaces, attributes, child_reply) = second;

    let properties = read_properties(connection, element, all).await?;
    let interfaces = field(
        interfaces.and_then(|message| message.body().deserialize::<Vec<String>>()),
        true,
    )?;
    let (extents, actions) = join(
        read_extents(connection, element, &interfaces),
        read_actions(connection, element, &interfaces),
    )
    .await;

    Ok(RawNode {
        element: element.clone(),
        properties,
        traits: NodeTraits {
            role: field(
                role.and_then(|message| message.body().deserialize::<u32>()),
                true,
            )?,
            role_name: field(
                role_name.and_then(|message| message.body().deserialize::<String>()),
                false,
            )?,
            states: field(
                state.and_then(|message| message.body().deserialize::<Vec<u32>>()),
                true,
            )?,
            interfaces,
        },
        attributes: field(
            attributes.and_then(|message| message.body().deserialize::<BTreeMap<String, String>>()),
            false,
        )?,
        extents: extents?,
        actions: actions?,
        children: read_children(child_reply)?,
    })
}

#[cfg(test)]
#[path = "node_read_tests.rs"]
mod tests;
