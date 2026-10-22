use std::collections::HashMap;

use agent_desktop_core::{AdapterError, Deadline};
use zbus::Connection;
use zbus::zvariant::{OwnedValue, Value};

use super::call;

const ITEM_INTERFACE: &str = "org.kde.StatusNotifierItem";
const NO_MENU: &str = "/NO_DBUSMENU";
const PASSIVE: &str = "Passive";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ItemProps {
    pub(crate) title: String,
    pub(crate) passive: bool,
    pub(crate) menu: Option<String>,
}

fn text(value: &OwnedValue) -> Option<String> {
    match &**value {
        Value::Str(text) => Some(text.as_str().to_owned()),
        Value::ObjectPath(path) => Some(path.as_str().to_owned()),
        _ => None,
    }
}

pub(crate) fn from_map(map: &HashMap<String, OwnedValue>) -> ItemProps {
    let read = |key: &str| map.get(key).and_then(text);
    ItemProps {
        title: read("Title").unwrap_or_default(),
        passive: read("Status").as_deref() == Some(PASSIVE),
        menu: read("Menu").filter(|path| path.starts_with('/') && path != NO_MENU),
    }
}

pub(crate) fn read(
    connection: &Connection,
    owner: &str,
    object_path: &str,
    deadline: Deadline,
) -> Result<ItemProps, AdapterError> {
    call::all_properties(connection, owner, object_path, ITEM_INTERFACE, deadline)
        .map(|map| from_map(&map))
}

#[cfg(test)]
#[path = "item_props_tests.rs"]
mod tests;
