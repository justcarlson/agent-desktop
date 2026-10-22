use std::collections::HashSet;
use std::path::PathBuf;

use agent_desktop_core::{AdapterError, Deadline, DeliverySemantics, ErrorCode};

use super::bus_process::owner_process_id;
use super::display_env::DisplayEnv;
use super::display_server::{DisplayServer, session_server};
use super::session_bus::SessionBus;

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

pub(super) fn is_display_owner(pid: u32, deadline: Deadline) -> Result<bool, AdapterError> {
    if owns_socket(pid, &display_socket()?)? {
        return Ok(true);
    }
    if matches!(session_server(), DisplayServer::X11(_)) {
        return super::x11::control::manager_pid(deadline)
            .ready()
            .map(|owner| owner == Some(pid))
            .ok_or_else(|| {
                unknown()
                    .with_platform_detail("The X root supporting-WM resource owner is unavailable")
            });
    }
    Ok(false)
}

fn gnome_backend(names: &[String], desktop: Option<&str>) -> Result<bool, AdapterError> {
    if names.iter().any(|name| {
        matches!(
            name.as_str(),
            "org.gnome.Shell" | "org.gnome.SessionManager"
        )
    }) {
        return Ok(true);
    }
    match desktop.filter(|desktop| !desktop.is_empty()) {
        Some(desktop)
            if desktop
                .split(':')
                .any(|name| name.eq_ignore_ascii_case("GNOME")) =>
        {
            Ok(true)
        }
        Some(_) => Err(unknown()),
        None => Ok(false),
    }
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
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").ok();
    let gnome = gnome_backend(&names, desktop.as_deref())?;
    for name in names.iter().filter(|name| {
        matches!(
            name.as_str(),
            "org.freedesktop.DBus"
                | "org.freedesktop.systemd1"
                | "org.freedesktop.ScreenSaver"
                | "org.a11y.Bus"
                | "org.freedesktop.portal.Desktop"
                | "org.freedesktop.Notifications"
                | "org.freedesktop.secrets"
        ) || name.starts_with("org.freedesktop.impl.portal.")
            || (gnome
                && (matches!(
                    name.as_str(),
                    "org.gnome.Shell" | "org.gnome.SessionManager"
                ) || name.starts_with("org.gnome.Shell.")
                    || name.starts_with("org.gnome.Mutter.")
                    || name.starts_with("org.gnome.SettingsDaemon.")))
    }) {
        match owner_process_id(connection, name).await {
            Ok(owner) if owner == pid => return Ok(true),
            Ok(_) => {}
            Err(zbus::Error::MethodError(name, _, _))
                if name.as_str() == "org.freedesktop.DBus.Error.NameHasNoOwner" => {}
            Err(_) => return Err(unknown()),
        }
    }
    if !gnome {
        for name in names.iter().filter(|name| !name.starts_with(':')) {
            match owner_process_id(connection, name).await {
                Ok(owner) if owner == pid => return Err(unknown()),
                Ok(_) => {}
                Err(zbus::Error::MethodError(name, _, _))
                    if name.as_str() == "org.freedesktop.DBus.Error.NameHasNoOwner" => {}
                Err(_) => return Err(unknown()),
            }
        }
    }
    Ok(false)
}

pub(crate) fn is_protected(pid: u32, deadline: Deadline) -> Result<bool, AdapterError> {
    if is_display_owner(pid, deadline)? {
        return Ok(true);
    }
    let connection = SessionBus::get(deadline)
        .map_err(|error| unknown().with_platform_detail(error.to_string()))?;
    super::within(deadline, async { service_owner(pid, &connection).await })
        .map_err(|error| error.with_disposition(DeliverySemantics::not_delivered()))
}

#[cfg(test)]
#[path = "protected_tests.rs"]
mod tests;
