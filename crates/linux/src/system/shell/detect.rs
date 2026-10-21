use std::collections::HashMap;
use std::future::Future;

use agent_desktop_core::{AdapterError, Deadline, ErrorCode};
use serde_json::json;
use zbus::Connection;
use zbus::zvariant::OwnedValue;

use self::errors::{tree_error, unreadable};
use super::identity::{SHELL_BUS_NAME, pick_shell_app, registry_apps};
use crate::system::session_bus::SessionBus;
use crate::system::{NAME_HAS_NO_OWNER, SERVICE_UNKNOWN, SharedBus, owner_process_id, within};

const SERVER_NAME: &str = "gnome-shell";
const NOTIFICATIONS: &str = "org.freedesktop.Notifications";
const NOTIFICATIONS_PATH: &str = "/org/freedesktop/Notifications";
const WATCHER: &str = "org.kde.StatusNotifierWatcher";
const WATCHER_PATH: &str = "/StatusNotifierWatcher";
const BUS_DAEMON: &str = "org.freedesktop.DBus";
const BUS_DAEMON_PATH: &str = "/org/freedesktop/DBus";
const EXTENSIONS: &str = "org.gnome.Shell.Extensions";
const EXTENSIONS_PATH: &str = "/org/gnome/Shell/Extensions";
const INDICATOR_EXTENSION: &str = "appindicator";
const EXTENSION_ENABLED: u32 = 1;

mod errors;
mod requirements;
pub(crate) use requirements::{require_daemon, require_watcher};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Fact {
    Present,
    Absent(String),
    Unreadable(String),
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShellEnvironment {
    pub(crate) daemon: Fact,
    pub(crate) tree: Fact,
    pub(crate) watcher: Fact,
    pub(crate) server: Option<String>,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Need {
    Daemon,
    Tree,
    Watcher,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CallFailure {
    NoService,
    Transport(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WatcherRead {
    NoOwner,
    Items,
}

pub(crate) fn failure_of(error: &zbus::Error) -> CallFailure {
    match error {
        zbus::Error::MethodError(name, _, _)
            if matches!(name.as_str(), SERVICE_UNKNOWN | NAME_HAS_NO_OWNER) =>
        {
            CallFailure::NoService
        }
        other => CallFailure::Transport(other.to_string()),
    }
}

fn call<T>(
    deadline: Deadline,
    read: impl Future<Output = Result<T, zbus::Error>>,
) -> Result<T, CallFailure> {
    match within(deadline, async {
        Ok(read.await.map_err(|error| failure_of(&error)))
    }) {
        Ok(answer) => answer,
        Err(error) => Err(CallFailure::Transport(error.message)),
    }
}

fn server_name(connection: &Connection, deadline: Deadline) -> Result<String, CallFailure> {
    call(deadline, async {
        let reply = connection
            .call_method(
                Some(NOTIFICATIONS),
                NOTIFICATIONS_PATH,
                Some(NOTIFICATIONS),
                "GetServerInformation",
                &(),
            )
            .await?;
        let (name, _vendor, _version, _spec) = reply
            .body()
            .deserialize::<(String, String, String, String)>()?;
        Ok(name)
    })
}

pub(crate) fn classify_daemon(answer: Result<String, CallFailure>) -> (Fact, Option<String>) {
    match answer {
        Ok(name) if name == SERVER_NAME => (Fact::Present, Some(name)),
        Ok(name) => (
            Fact::Absent(format!(
                "the notification service is '{name}', not {SERVER_NAME}"
            )),
            Some(name),
        ),
        Err(CallFailure::NoService) => (
            Fact::Absent("no service owns org.freedesktop.Notifications".to_owned()),
            None,
        ),
        Err(CallFailure::Transport(detail)) => (Fact::Unreadable(detail), None),
    }
}

pub(crate) fn classify_tree(
    owner: Result<u32, CallFailure>,
    registered: Option<Result<bool, CallFailure>>,
) -> Fact {
    match (owner, registered) {
        (Err(CallFailure::NoService), _) => {
            Fact::Absent("org.gnome.Shell has no owner on the session bus".to_owned())
        }
        (Err(CallFailure::Transport(detail)), _) => Fact::Unreadable(detail),
        (Ok(_), Some(Ok(true))) => Fact::Present,
        (Ok(_), Some(Ok(false))) => {
            Fact::Absent("the shell process is not on the accessibility registry".to_owned())
        }
        (Ok(_), Some(Err(CallFailure::NoService))) => {
            Fact::Absent("the accessibility registry is not answering".to_owned())
        }
        (Ok(_), Some(Err(CallFailure::Transport(detail)))) => Fact::Unreadable(detail),
        (Ok(_), None) => Fact::Unreadable("the registry was not read".to_owned()),
    }
}

pub(crate) fn detect(bus: &SharedBus, deadline: Deadline) -> Fact {
    let session = match SessionBus::get(deadline) {
        Ok(connection) => connection,
        Err(error) => return Fact::Unreadable(error.message),
    };
    let owner = call(deadline, owner_process_id(&session, SHELL_BUS_NAME));
    let registered = owner.as_ref().ok().map(|pid| {
        let accessibility = bus
            .get(deadline)
            .map_err(|error| CallFailure::Transport(error.message))?;
        within(deadline, registry_apps(accessibility.inner()))
            .map(|apps| pick_shell_app(&apps, *pid).is_some())
            .map_err(|error| CallFailure::Transport(error.message))
    });
    classify_tree(owner, registered)
}

pub(crate) fn require_tree(bus: &SharedBus, deadline: Deadline) -> Result<(), AdapterError> {
    tree_error(&detect(bus, deadline))
}

fn watcher(connection: &Connection, deadline: Deadline) -> Result<WatcherRead, CallFailure> {
    call(deadline, async {
        let owned = connection
            .call_method(
                Some(BUS_DAEMON),
                BUS_DAEMON_PATH,
                Some(BUS_DAEMON),
                "NameHasOwner",
                &(WATCHER,),
            )
            .await?
            .body()
            .deserialize::<bool>()?;
        if !owned {
            return Ok(WatcherRead::NoOwner);
        }
        let reply = connection
            .call_method(
                Some(WATCHER),
                WATCHER_PATH,
                Some("org.freedesktop.DBus.Properties"),
                "Get",
                &(WATCHER, "RegisteredStatusNotifierItems"),
            )
            .await?;
        reply.body().deserialize::<OwnedValue>()?;
        Ok(WatcherRead::Items)
    })
}

fn indicator_extension(connection: &Connection, deadline: Deadline) -> Option<String> {
    let listed = call(deadline, async {
        let reply = connection
            .call_method(
                Some(SHELL_BUS_NAME),
                EXTENSIONS_PATH,
                Some(EXTENSIONS),
                "ListExtensions",
                &(),
            )
            .await?;
        reply
            .body()
            .deserialize::<HashMap<String, HashMap<String, OwnedValue>>>()
    })
    .ok()?;
    Some(extension_state(&listed))
}

pub(crate) fn classify_watcher(
    read: Result<WatcherRead, CallFailure>,
    extension: Option<String>,
) -> Fact {
    match read {
        Ok(WatcherRead::Items) => Fact::Present,
        Ok(WatcherRead::NoOwner) => {
            let state = extension.unwrap_or_else(|| "extension state unknown".to_owned());
            Fact::Absent(format!("no tray watcher is running ({state})"))
        }
        Err(CallFailure::NoService) => {
            Fact::Unreadable("the tray watcher left the bus while it was read".to_owned())
        }
        Err(CallFailure::Transport(detail)) => Fact::Unreadable(detail),
    }
}

fn daemon_fact_error(daemon: &Fact, server: Option<&str>) -> Result<(), AdapterError> {
    if let Some(server) = server.filter(|name| *name != SERVER_NAME) {
        return Err(AdapterError::new(
            ErrorCode::PlatformNotSupported,
            format!("The notification service is '{server}'; only GNOME Shell is supported"),
        )
        .with_suggestion(
            "Use a GNOME Shell session, or install a GNOME-compatible notification service",
        )
        .with_details(json!({ "daemon": "unsupported", "server": server })));
    }
    match daemon {
        Fact::Present => Ok(()),
        Fact::Absent(why) => Err(AdapterError::new(
            ErrorCode::PlatformNotSupported,
            "No notification service is running in this session",
        )
        .with_suggestion("Install and start a GNOME-compatible notification service")
        .with_details(json!({ "daemon": "absent", "detail": why }))),
        Fact::Unreadable(why) => Err(unreadable("notification service", why, "daemon")),
    }
}

fn watcher_fact_error(watcher: &Fact) -> Result<(), AdapterError> {
    match watcher {
        Fact::Present => Ok(()),
        Fact::Absent(why) => Err(AdapterError::new(
            ErrorCode::PlatformNotSupported,
            "No system tray watcher is running in this session",
        )
        .with_suggestion("Enable the AppIndicator extension so tray items register")
        .with_details(json!({ "watcher": "absent", "detail": why }))),
        Fact::Unreadable(why) => Err(unreadable("system tray watcher", why, "watcher")),
    }
}

#[cfg(test)]
pub(crate) fn require(env: &ShellEnvironment, need: Need) -> Result<(), AdapterError> {
    match need {
        Need::Daemon => daemon_fact_error(&env.daemon, env.server.as_deref()),
        Need::Tree => tree_error(&env.tree),
        Need::Watcher => watcher_fact_error(&env.watcher),
    }
}

pub(crate) fn extension_state(listed: &HashMap<String, HashMap<String, OwnedValue>>) -> String {
    let found = listed
        .iter()
        .find(|(uuid, _)| uuid.contains(INDICATOR_EXTENSION));
    match found {
        None => "the AppIndicator extension is not installed".to_owned(),
        Some((_, properties)) => {
            let state = properties
                .get("state")
                .and_then(|value| value.downcast_ref::<f64>().ok());
            match state {
                Some(state) if state as u32 == EXTENSION_ENABLED => {
                    "the AppIndicator extension is installed and enabled".to_owned()
                }
                _ => "the AppIndicator extension is installed but not enabled".to_owned(),
            }
        }
    }
}

#[cfg(test)]
#[path = "detect_tests.rs"]
mod tests;
