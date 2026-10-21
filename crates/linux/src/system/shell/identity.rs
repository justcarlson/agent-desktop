use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use agent_desktop_core::{AdapterError, Deadline, DeliverySemantics, ErrorCode};
use serde_json::json;
use zbus::Connection;

use super::light::{LiveNodes, Nodes};
use crate::system::session_bus::SessionBus;
use crate::system::{
    REGISTRY_ROOT_PATH, REGISTRY_SERVICE, SharedBus, owner_process_id, process_facts, within,
};
use crate::tree::role_numbers::WINDOW;
use crate::tree::{AtspiElement, list_children};

pub(crate) const SHELL_BUS_NAME: &str = "org.gnome.Shell";
const SHELL_COMM: &str = "gnome-shell";
const WINDOW_RETRIES: usize = 10;
const WINDOW_RETRY_GAP: Duration = Duration::from_millis(100);
const WINDOW_BUDGET: Duration = Duration::from_millis(2_000);
const OUTAGE_MEMORY: Duration = Duration::from_millis(3_000);

static LAST_MISSING: Mutex<Option<Instant>> = Mutex::new(None);

/// A window that has just been missing for a whole retry is still missing a
/// moment later, so the next lookup in the same outage probes once instead of
/// spending the retry again and again until the command's budget is gone.
pub(super) fn retries_after(last_missing: Option<Instant>, now: Instant) -> usize {
    match last_missing {
        Some(at) if now.saturating_duration_since(at) < OUTAGE_MEMORY => 0,
        _ => WINDOW_RETRIES,
    }
}

/// The shell's accessibility application and its content window. `bus_name`
/// is the unique name on the accessibility bus, `pid` the process behind it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShellRoot {
    pub(crate) bus_name: String,
    pub(crate) path: String,
    pub(crate) pid: u32,
}

static KNOWN: Mutex<Option<ShellRoot>> = Mutex::new(None);

pub(super) fn remember(root: &ShellRoot) {
    *KNOWN.lock().unwrap_or_else(PoisonError::into_inner) = Some(root.clone());
}

pub(crate) fn is_shell_comm(comm: &str) -> bool {
    comm == SHELL_COMM
}

/// Whether an accessibility bus name or a process belongs to the shell. It
/// reads no bus: the name found by the last root lookup, then the command
/// name of the process.
pub(crate) fn is_shell_owner(bus_name: &str, pid: u32) -> bool {
    if KNOWN
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_ref()
        .is_some_and(|root| root.bus_name == bus_name)
    {
        return true;
    }
    process_facts::read(pid).is_some_and(|facts| is_shell_comm(&facts.comm))
}

/// The registry application owned by the shell process, from the processes
/// behind the registry's children.
pub(crate) fn pick_shell_app(
    apps: &[(AtspiElement, u32)],
    shell_pid: u32,
) -> Option<&AtspiElement> {
    apps.iter()
        .find(|(_, pid)| *pid == shell_pid)
        .map(|(app, _)| app)
}

/// The content window among an application's children: the first child the
/// reader says is a window.
pub(crate) fn content_window<N: Nodes>(
    nodes: &N,
    app: &AtspiElement,
) -> Result<Option<AtspiElement>, AdapterError> {
    let root = nodes.read(app)?;
    for child in &root.children {
        let light = match nodes.read(child) {
            Ok(light) => light,
            Err(error) if error.code == ErrorCode::ElementNotFound => continue,
            Err(error) => return Err(error),
        };
        if light.role == WINDOW {
            return Ok(Some(light.element));
        }
    }
    Ok(None)
}

/// Runs `probe` until it answers, at most `1 + attempts` times, pausing
/// between tries and stopping at the deadline. The shell's content window
/// drops out of the registry for a moment after a mutation, so a miss right
/// then is not yet an absent shell.
pub(super) fn retry_until<T>(
    attempts: usize,
    deadline: Deadline,
    pause: &mut dyn FnMut(Duration),
    mut probe: impl FnMut() -> Option<T>,
) -> Option<T> {
    for attempt in 0..=attempts {
        if let Some(found) = probe() {
            return Some(found);
        }
        if attempt == attempts || deadline.is_expired() {
            return None;
        }
        pause(WINDOW_RETRY_GAP);
    }
    None
}

/// The shell is on the bus but its content window is out of the registry,
/// which happens for seconds after a change in the message list while the
/// process stays up. It is named apart from an absent shell and answers
/// within about a second, so a retry a little later is the right move.
pub(super) fn window_missing() -> AdapterError {
    AdapterError::new(
        ErrorCode::AppNotFound,
        "The desktop shell's window is missing from the accessibility registry for the moment",
    )
    .with_suggestion("Retry the command in a few seconds; the shell process is running")
    .with_details(json!({
        "shell_tree": "window_missing",
        "retryable": true,
    }))
    .with_disposition(DeliverySemantics::not_delivered())
}

pub(crate) fn tree_absent(detail: &str) -> AdapterError {
    AdapterError::new(
        ErrorCode::AppNotFound,
        "The GNOME Shell accessibility capability is absent from this session",
    )
    .with_suggestion(
        "Use a GNOME Shell session exposing org.gnome.Shell and its accessibility tree",
    )
    .with_details(json!({ "shell_tree": "absent", "detail": detail }))
}

pub(super) async fn shell_owner(session: &Connection) -> Result<u32, zbus::Error> {
    owner_process_id(session, SHELL_BUS_NAME)
        .await
        .inspect_err(|error| {
            if crate::system::session_bus::is_transport_failure(error) {
                SessionBus::forget(session);
            }
        })
}

async fn shell_pid(session: &Connection) -> Result<Option<u32>, AdapterError> {
    match shell_owner(session).await {
        Ok(pid) => Ok(Some(pid)),
        Err(zbus::Error::MethodError(name, _, _))
            if name.as_str() == crate::system::NAME_HAS_NO_OWNER
                || name.as_str() == crate::system::SERVICE_UNKNOWN =>
        {
            Ok(None)
        }
        Err(error) => Err(crate::system::classify(&error)),
    }
}

pub(super) async fn registry_apps(
    accessibility: &Connection,
) -> Result<Vec<(AtspiElement, u32)>, AdapterError> {
    let registry = AtspiElement::new(REGISTRY_SERVICE, REGISTRY_ROOT_PATH, 0);
    let children = list_children(accessibility, &registry).await?;
    let mut apps = Vec::with_capacity(children.len());
    for (bus, path) in children {
        if let Ok(pid) = owner_process_id(accessibility, &bus).await {
            apps.push((AtspiElement::new(bus, path, pid), pid));
        }
    }
    Ok(apps)
}

pub(crate) fn find_root(bus: &SharedBus, deadline: Deadline) -> Result<ShellRoot, AdapterError> {
    let session = SessionBus::get(deadline)?;
    let accessibility = bus.get(deadline)?.inner().clone();
    let pid = within(deadline, shell_pid(&session))?
        .ok_or_else(|| tree_absent("org.gnome.Shell has no owner on the session bus"))?;
    let apps = within(deadline, registry_apps(&accessibility))?;
    let app = pick_shell_app(&apps, pid)
        .ok_or_else(|| tree_absent("no registry application belongs to the shell process"))?;
    if !process_facts::read(pid).is_some_and(|facts| is_shell_comm(&facts.comm)) {
        return Err(tree_absent("the shell process is not a gnome-shell"));
    }
    if crate::system::proc_stat::is_stopped(pid) {
        return Err(AdapterError::app_unresponsive(SHELL_COMM));
    }
    let probe_deadline = deadline.capped(WINDOW_BUDGET);
    let nodes = LiveNodes::new(accessibility, probe_deadline);
    let last = *LAST_MISSING.lock().unwrap_or_else(PoisonError::into_inner);
    let retries = retries_after(last, Instant::now());
    let window = retry_until(retries, probe_deadline, &mut std::thread::sleep, || {
        content_window(&nodes, app).transpose()
    })
    .transpose()?
    .ok_or_else(|| {
        *LAST_MISSING.lock().unwrap_or_else(PoisonError::into_inner) = Some(Instant::now());
        window_missing()
    })?;
    let root = ShellRoot {
        bus_name: window.bus_name().to_owned(),
        path: window.path().to_owned(),
        pid,
    };
    remember(&root);
    Ok(root)
}

#[cfg(test)]
#[path = "identity_tests.rs"]
mod tests;
