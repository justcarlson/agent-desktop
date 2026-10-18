use std::collections::{BTreeMap, HashMap};

use agent_desktop_core::{AdapterError, ErrorCode};
use futures_util::future::{join, join_all, join3, join4};
use zbus::zvariant::OwnedValue;
use zbus::{Connection, Message};

use super::{AtspiElement, ChildAddress, Extents, Field, NodeProperties, NodeTraits, RawNode};

const PROPERTIES_INTERFACE: &str = "org.freedesktop.DBus.Properties";
const ACCESSIBLE_INTERFACE: &str = "org.a11y.atspi.Accessible";
const COMPONENT_INTERFACE: &str = "org.a11y.atspi.Component";
const ACTION_INTERFACE: &str = "org.a11y.atspi.Action";
const UNKNOWN_OBJECT: &str = "org.freedesktop.DBus.Error.UnknownObject";
const UNKNOWN_METHOD: &str = "org.freedesktop.DBus.Error.UnknownMethod";
const SERVICE_UNKNOWN: &str = "org.freedesktop.DBus.Error.ServiceUnknown";
const NAME_HAS_NO_OWNER: &str = "org.freedesktop.DBus.Error.NameHasNoOwner";
const NULL_PATH: &str = "/org/a11y/atspi/null";
const SCREEN_COORDINATES: u32 = 0;

type Reply = Result<Message, zbus::Error>;

async fn call(
    connection: &Connection,
    element: &AtspiElement,
    interface: &'static str,
    method: &'static str,
) -> Reply {
    connection
        .call_method(
            Some(element.bus_name()),
            element.path(),
            Some(interface),
            method,
            &(),
        )
        .await
}

async fn call_get_all(connection: &Connection, element: &AtspiElement) -> Reply {
    connection
        .call_method(
            Some(element.bus_name()),
            element.path(),
            Some(PROPERTIES_INTERFACE),
            "GetAll",
            &(ACCESSIBLE_INTERFACE,),
        )
        .await
}

async fn call_get_extents(connection: &Connection, element: &AtspiElement) -> Reply {
    connection
        .call_method(
            Some(element.bus_name()),
            element.path(),
            Some(COMPONENT_INTERFACE),
            "GetExtents",
            &(SCREEN_COORDINATES,),
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

fn property_string(map: &HashMap<String, OwnedValue>, key: &str) -> Field<String> {
    match map.get(key) {
        None => Field::Absent,
        Some(value) => match value.downcast_ref::<&str>() {
            Ok(text) => Field::Present(text.to_owned()),
            Err(error) => Field::Failed(error.to_string()),
        },
    }
}

fn property_count(map: &HashMap<String, OwnedValue>, key: &str) -> Field<i32> {
    match map.get(key) {
        None => Field::Absent,
        Some(value) => match value.downcast_ref::<i32>() {
            Ok(count) => Field::Present(count),
            Err(error) => Field::Failed(error.to_string()),
        },
    }
}

fn from_map(map: &HashMap<String, OwnedValue>) -> NodeProperties {
    NodeProperties {
        name: property_string(map, "Name"),
        description: property_string(map, "Description"),
        child_count: property_count(map, "ChildCount"),
        accessible_id: property_string(map, "AccessibleId"),
    }
}

async fn call_get_property(
    connection: &Connection,
    element: &AtspiElement,
    property: &'static str,
) -> Result<OwnedValue, zbus::Error> {
    connection
        .call_method(
            Some(element.bus_name()),
            element.path(),
            Some(PROPERTIES_INTERFACE),
            "Get",
            &(ACCESSIBLE_INTERFACE, property),
        )
        .await?
        .body()
        .deserialize::<OwnedValue>()
}

async fn get_one(
    connection: &Connection,
    element: &AtspiElement,
    property: &'static str,
) -> Result<HashMap<String, OwnedValue>, AdapterError> {
    let read = field(call_get_property(connection, element, property).await, true)?;
    Ok(match read {
        Field::Present(value) => HashMap::from([(property.to_owned(), value)]),
        Field::Absent | Field::Failed(_) => HashMap::new(),
    })
}

/// Toolkits that do not implement `GetAll` for the accessible interface (Qt
/// answers `UnknownInterface`) still answer each property individually.
async fn properties_one_by_one(
    connection: &Connection,
    element: &AtspiElement,
) -> Result<NodeProperties, AdapterError> {
    let (name, description, count, id) = join4(
        get_one(connection, element, "Name"),
        get_one(connection, element, "Description"),
        get_one(connection, element, "ChildCount"),
        get_one(connection, element, "AccessibleId"),
    )
    .await;
    let mut merged = HashMap::new();
    let mut failed = Vec::new();
    for (key, answer) in [
        ("Name", name?),
        ("Description", description?),
        ("ChildCount", count?),
        ("AccessibleId", id?),
    ] {
        if answer.is_empty() {
            failed.push(key);
        }
        merged.extend(answer);
    }
    let mut properties = from_map(&merged);
    for key in failed {
        let detail = format!("Properties.Get({key}) failed");
        match key {
            "Name" => properties.name = Field::Failed(detail),
            "Description" => properties.description = Field::Failed(detail),
            "ChildCount" => properties.child_count = Field::Failed(detail),
            _ => properties.accessible_id = Field::Failed(detail),
        }
    }
    Ok(properties)
}

async fn properties(
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

fn children(reply: Reply) -> Result<Field<Vec<ChildAddress>>, AdapterError> {
    let raw = field(
        reply.and_then(|message| {
            message
                .body()
                .deserialize::<Vec<(String, zbus::zvariant::OwnedObjectPath)>>()
        }),
        true,
    )?;
    Ok(match raw {
        Field::Present(entries) => Field::Present(
            entries
                .into_iter()
                .filter(|(bus, path)| !bus.is_empty() && path.as_str() != NULL_PATH)
                .map(|(bus, path)| (bus, path.to_string()))
                .collect(),
        ),
        Field::Absent => Field::Absent,
        Field::Failed(detail) => Field::Failed(detail),
    })
}

fn advertises(interfaces: &Field<Vec<String>>, name: &str) -> bool {
    interfaces
        .present()
        .is_some_and(|list| list.iter().any(|entry| entry == name))
}

async fn read_extents(
    connection: &Connection,
    element: &AtspiElement,
    interfaces: &Field<Vec<String>>,
) -> Result<Field<Extents>, AdapterError> {
    if !advertises(interfaces, COMPONENT_INTERFACE) {
        return Ok(Field::Absent);
    }
    let reply = call_get_extents(connection, element).await;
    field(
        reply.and_then(|message| message.body().deserialize::<(i32, i32, i32, i32)>()),
        false,
    )
    .map(|read| match read {
        Field::Present((x, y, width, height)) => Field::Present(Extents {
            x,
            y,
            width,
            height,
        }),
        Field::Absent => Field::Absent,
        Field::Failed(detail) => Field::Failed(detail),
    })
}

async fn action_name(
    connection: &Connection,
    element: &AtspiElement,
    index: i32,
    listed: String,
) -> String {
    let reply = connection
        .call_method(
            Some(element.bus_name()),
            element.path(),
            Some(ACTION_INTERFACE),
            "GetName",
            &(index,),
        )
        .await;
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
    if !advertises(interfaces, ACTION_INTERFACE) {
        return Ok(Field::Absent);
    }
    let reply = call(connection, element, ACTION_INTERFACE, "GetActions").await;
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
            join_all(
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
            call_get_all(connection, element),
            call(connection, element, ACCESSIBLE_INTERFACE, "GetRole"),
            call(connection, element, ACCESSIBLE_INTERFACE, "GetRoleName"),
            call(connection, element, ACCESSIBLE_INTERFACE, "GetState"),
        ),
        join3(
            call(connection, element, ACCESSIBLE_INTERFACE, "GetInterfaces"),
            call(connection, element, ACCESSIBLE_INTERFACE, "GetAttributes"),
            call(connection, element, ACCESSIBLE_INTERFACE, "GetChildren"),
        ),
    )
    .await;
    let (all, role, role_name, state) = first;
    let (interfaces, attributes, child_reply) = second;

    let properties = properties(connection, element, all).await?;
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
        children: children(child_reply)?,
    })
}
