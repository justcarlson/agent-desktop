use std::collections::HashSet;
use std::path::PathBuf;

use agent_desktop_core::{AdapterError, Deadline, DeliverySemantics, ErrorCode};

use crate::system::bus::bus_process::owner_process_id;
use crate::system::bus::session_bus::SessionBus;
use crate::system::display::display_env::DisplayEnv;
use crate::system::display::display_server::{DisplayServer, session_server};
use crate::system::process::process_facts;
use crate::system::process::process_lookup::Lookup;

fn unknown() -> AdapterError {
    AdapterError::new(
        ErrorCode::ActionNotSupported,
        "The session process ownership could not be proven",
    )
    .with_disposition(DeliverySemantics::not_delivered())
    .with_suggestion(
        "Run inside the target desktop session with readable process and session-bus ownership.",
    )
}

fn display_socket() -> Result<PathBuf, AdapterError> {
    match session_server() {
        DisplayServer::Wayland => {
            let env = DisplayEnv::from_process();
            let name = env.wayland.ok_or_else(unknown)?;
            let path = PathBuf::from(name);
            if path.is_absolute() {
                Ok(path)
            } else {
                Ok(env.runtime_dir.ok_or_else(unknown)?.join(path))
            }
        }
        DisplayServer::X11(display) => {
            let number = display.strip_prefix(':').ok_or_else(unknown)?;
            let number = number.split('.').next().ok_or_else(unknown)?;
            if number.is_empty() || !number.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(unknown());
            }
            Ok(PathBuf::from(format!("/tmp/.X11-unix/X{number}")))
        }
        DisplayServer::Unavailable(_) => Err(unknown()),
    }
}

fn owns_socket(pid: u32, socket: &std::path::Path) -> Result<bool, AdapterError> {
    let table = std::fs::read_to_string("/proc/net/unix").map_err(|_| unknown())?;
    let path = socket.to_str().ok_or_else(unknown)?;
    let abstract_path = format!("@{path}");
    let inodes: HashSet<String> = table
        .lines()
        .skip(1)
        .filter_map(|line| {
            let words: Vec<_> = line.split_whitespace().collect();
            let bound = words.get(7..)?.join(" ");
            let flags = u32::from_str_radix(words.get(3)?, 16).ok()?;
            ((bound == path || bound == abstract_path) && flags & 0x0001_0000 != 0)
                .then(|| format!("socket:[{}]", words[6]))
        })
        .collect();
    if inodes.is_empty() {
        return Err(unknown());
    }
    let fds = std::fs::read_dir(format!("/proc/{pid}/fd")).map_err(|_| unknown())?;
    for fd in fds {
        let fd = fd.map_err(|_| unknown())?;
        match std::fs::read_link(fd.path()) {
            Ok(link) if inodes.contains(link.to_string_lossy().as_ref()) => return Ok(true),
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(unknown()),
        }
    }
    Ok(false)
}

pub(in crate::system) fn is_display_owner(
    pid: u32,
    deadline: Deadline,
) -> Result<bool, AdapterError> {
    if owns_socket(pid, &display_socket()?)? {
        return Ok(true);
    }
    if matches!(session_server(), DisplayServer::X11(_)) {
        return crate::system::x11::control::manager_pid(deadline)
            .ready()
            .map(|owner| owner == Some(pid))
            .ok_or_else(|| {
                unknown()
                    .with_platform_detail("The X root supporting-WM resource owner is unavailable")
            });
    }
    Ok(false)
}

async fn service_owner(pid: u32, connection: &zbus::Connection) -> Result<bool, AdapterError> {
    let reply = connection
        .call_method(
            Some("org.freedesktop.DBus"),
            "/org/freedesktop/DBus",
            Some("org.freedesktop.DBus"),
            "ListNames",
            &(),
        )
        .await
        .map_err(|_| unknown())?;
    let names = reply
        .body()
        .deserialize::<Vec<String>>()
        .map_err(|_| unknown())?;
    for name in names.iter().filter(|name| !name.starts_with(':')) {
        match owner_process_id(connection, name).await {
            Ok(owner) if owner == pid => return Ok(true),
            Ok(_) => {}
            Err(zbus::Error::MethodError(name, _, _))
                if name.as_str() == "org.freedesktop.DBus.Error.NameHasNoOwner" => {}
            Err(_) => return Err(unknown()),
        }
    }
    Ok(false)
}

pub(crate) fn is_protected(
    bus: &crate::system::SharedBus,
    pid: u32,
    instance: &str,
    deadline: Deadline,
) -> Result<bool, AdapterError> {
    let instance_matches =
        || matches!(process_facts::lookup(pid), Lookup::Found(facts) if facts.instance == instance);
    if !instance_matches() {
        return Err(unknown());
    }
    let display_owner = is_display_owner(pid, deadline)?;
    if !instance_matches() {
        return Err(unknown());
    }
    if display_owner {
        return Ok(true);
    }
    let registry = crate::system::registry::read_registry_for(bus, Some(pid), deadline)
        .map_err(|error| unknown().with_platform_detail(error.to_string()))?;
    if !instance_matches() {
        return Err(unknown());
    }
    if registry
        .apps
        .iter()
        .any(|app| app.root.pid() == pid && app.answered)
    {
        return Ok(false);
    }
    let connection = SessionBus::get(deadline)
        .map_err(|error| unknown().with_platform_detail(error.to_string()))?;
    let owner = crate::system::within(deadline, async { service_owner(pid, &connection).await })
        .map_err(|error| error.with_disposition(DeliverySemantics::not_delivered()))?;
    if !instance_matches() {
        return Err(unknown());
    }
    Ok(owner)
}

#[cfg(test)]
#[path = "protected_tests.rs"]
mod tests;
