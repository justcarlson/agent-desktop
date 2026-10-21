use std::collections::{BTreeMap, HashMap};

use agent_desktop_core::AdapterError;
use futures_util::future::{join, join3, join4};
use zbus::export::serde::Serialize;
use zbus::zvariant::{DynamicDeserialize, DynamicType, OwnedValue};
use zbus::{Connection, Message};

use super::absence::gone_error;
use super::bounded::collect_bounded;
use super::liveness::Liveness;
use super::{AtspiElement, ChildAddress, Extents, Field, NodeProperties, NodeTraits, RawNode};
use crate::system::{ACCESSIBLE_INTERFACE, PROPERTIES_INTERFACE};

pub(crate) const COMPONENT_INTERFACE: &str = "org.a11y.atspi.Component";
const ACTION_INTERFACE: &str = "org.a11y.atspi.Action";
const ACTION_NAMES_IN_FLIGHT: usize = 4;
pub(super) const NAME: &str = "Name";
const DESCRIPTION: &str = "Description";
const CHILD_COUNT: &str = "ChildCount";
const ACCESSIBLE_ID: &str = "AccessibleId";
const NULL_PATH: &str = "/org/a11y/atspi/null";
const SCREEN_COORDINATES: u32 = 0;

pub(super) type Reply = Result<Message, zbus::Error>;

fn deserialize<T>(reply: Reply) -> Result<T, zbus::Error>
where
    T: for<'de> DynamicDeserialize<'de>,
{
    reply?.body().deserialize()
}

pub(crate) async fn call<B>(
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

/// A read's outcome as a field. An error that shows the application or the
/// object gone fails the node read; any other failure, including a name from
/// the suspect family on a node that still answers, stays on its field.
fn field<T>(result: Result<T, zbus::Error>, liveness: Liveness) -> Result<Field<T>, AdapterError> {
    match result {
        Ok(value) => Ok(Field::Present(value)),
        Err(error) if crate::system::is_connection_failure(&error) => {
            Err(crate::system::classify(&error))
        }
        Err(error) => match gone_error(&error, liveness) {
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
    liveness: Liveness,
) -> Result<Field<T>, AdapterError> {
    let reply = call(
        connection,
        element,
        PROPERTIES_INTERFACE,
        "Get",
        &(ACCESSIBLE_INTERFACE, key),
    )
    .await;
    let read = field(deserialize::<OwnedValue>(reply), liveness)?;
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
    liveness: Liveness,
) -> Result<NodeProperties, AdapterError> {
    let (name, description, child_count, accessible_id) = join4(
        get_one(connection, element, NAME, string_value, liveness),
        get_one(connection, element, DESCRIPTION, string_value, liveness),
        get_one(connection, element, CHILD_COUNT, count_value, liveness),
        get_one(connection, element, ACCESSIBLE_ID, string_value, liveness),
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
    liveness: Liveness,
) -> Result<NodeProperties, AdapterError> {
    let map = field(deserialize::<HashMap<String, OwnedValue>>(reply), liveness)?;
    match map {
        Field::Present(map) => Ok(from_map(&map)),
        Field::Absent | Field::Failed(_) => {
            properties_one_by_one(connection, element, liveness).await
        }
    }
}

pub(super) fn read_children(
    reply: Reply,
    liveness: Liveness,
) -> Result<Field<Vec<ChildAddress>>, AdapterError> {
    let raw = field(
        deserialize::<Vec<(String, zbus::zvariant::OwnedObjectPath)>>(reply),
        liveness,
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
    liveness: Liveness,
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
    field(deserialize::<(i32, i32, i32, i32)>(reply), liveness).map(|read| {
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
    deserialize::<String>(reply)
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
    liveness: Liveness,
) -> Result<Field<Vec<String>>, AdapterError> {
    if let Some(skipped) = skipped_without(interfaces, ACTION_INTERFACE) {
        return Ok(skipped);
    }
    let reply = call(connection, element, ACTION_INTERFACE, "GetActions", &()).await;
    let listed = field(
        deserialize::<Vec<(String, String, String)>>(reply),
        liveness,
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
/// actions follow only when the interface list names them. The node's own
/// `GetRole` outcome, which every live node answers, decides what a failure
/// of any other call means: an error that shows the application or the object
/// gone fails the read; any other failure stays on the field it hit. The
/// second value is `Alive`, or `TimedOut` when the role call did not answer
/// in time, which says nothing about the object.
pub(crate) async fn read_node_with_liveness(
    connection: &Connection,
    element: &AtspiElement,
) -> Result<(RawNode, Liveness), AdapterError> {
    let accessible =
        |method: &'static str| call(connection, element, ACCESSIBLE_INTERFACE, method, &());
    let (first, second) = join(
        join4(
            call(
                connection,
                element,
                PROPERTIES_INTERFACE,
                "GetAll",
                &(ACCESSIBLE_INTERFACE,),
            ),
            accessible("GetRole"),
            accessible("GetRoleName"),
            accessible("GetState"),
        ),
        join3(
            accessible("GetInterfaces"),
            accessible("GetAttributes"),
            accessible("GetChildren"),
        ),
    )
    .await;
    let (all, role, role_name, state) = first;
    let (interfaces, attributes, child_reply) = second;

    let role = deserialize::<u32>(role);
    let liveness = Liveness::of(role.as_ref().err());
    let role = field(role, liveness)?;
    let properties = read_properties(connection, element, all, liveness).await?;
    let interfaces = field(deserialize::<Vec<String>>(interfaces), liveness)?;
    let (extents, actions) = join(
        read_extents(connection, element, &interfaces, liveness),
        read_actions(connection, element, &interfaces, liveness),
    )
    .await;

    let node = RawNode {
        element: element.clone(),
        properties,
        traits: NodeTraits {
            role,
            role_name: field(deserialize::<String>(role_name), liveness)?,
            states: field(deserialize::<Vec<u32>>(state), liveness)?,
            interfaces,
        },
        attributes: field(
            deserialize::<BTreeMap<String, String>>(attributes),
            liveness,
        )?,
        extents: extents?,
        actions: actions?,
        children: read_children(child_reply, liveness)?,
    };
    Ok((node, liveness))
}

pub(crate) async fn read_node(
    connection: &Connection,
    element: &AtspiElement,
) -> Result<RawNode, AdapterError> {
    read_node_with_liveness(connection, element)
        .await
        .map(|(node, _)| node)
}

#[cfg(test)]
#[path = "node_read_tests.rs"]
mod tests;
