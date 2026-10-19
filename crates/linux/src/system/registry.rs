use std::time::Duration;

use agent_desktop_core::{AdapterError, Deadline};
use futures_util::future::join;
use futures_util::stream::{self, StreamExt};
use serde_json::Value;
use zbus::Connection;
use zbus::zvariant::OwnedValue;

use super::bus_slice::within_slice;
use super::proc_stat::ProcState;
use super::process_facts;
use super::registry_app::{RegistryApp, assemble, skipped};
use super::toolkit_id::ToolkitId;
use super::{
    ACCESSIBLE_INTERFACE, PROPERTIES_INTERFACE, REGISTRY_ROOT_PATH, REGISTRY_SERVICE, SharedBus,
    process_id, within,
};
use crate::tree::toolkit_trust::read_toolkit;
use crate::tree::{AtspiElement, list_children};

pub(crate) const ROOT_SLICE: Duration = Duration::from_millis(300);
const APPS_IN_FLIGHT: usize = 32;

/// Every application on the registry, and those that could not be read.
pub(crate) struct Registry {
    pub(crate) apps: Vec<RegistryApp>,
    pub(crate) skipped: Vec<Value>,
}

enum AppRead {
    Listed(Box<RegistryApp>),
    Skipped(Value),
}

async fn root_name(connection: &Connection, root: &AtspiElement) -> Option<Option<String>> {
    let reply = connection
        .call_method(
            Some(root.bus_name()),
            root.path(),
            Some(PROPERTIES_INTERFACE),
            "Get",
            &(ACCESSIBLE_INTERFACE, "Name"),
        )
        .await
        .ok()?;
    let value = reply.body().deserialize::<OwnedValue>().ok()?;
    Some(value.downcast_ref::<&str>().ok().map(str::to_owned))
}

pub(crate) async fn toolkit(connection: &Connection, root: &AtspiElement) -> ToolkitId {
    read_toolkit(connection, root).await
}

async fn read_app(connection: &Connection, bus: String, path: String) -> AppRead {
    let pid = match process_id(connection, bus.clone()).await.1 {
        Ok(pid) => pid,
        Err(error) => return AppRead::Skipped(skipped(&bus, None, &error.message)),
    };
    let Some((facts, state)) = process_facts::read_with_state(pid) else {
        return AppRead::Skipped(skipped(&bus, Some(pid), "the process has exited"));
    };
    let root = AtspiElement::new(bus, path, pid);
    if state == ProcState::Stopped {
        return AppRead::Listed(Box::new(assemble(root, facts, None, ToolkitId::default())));
    }
    let (name, toolkit) = join(
        within_slice(ROOT_SLICE, root_name(connection, &root)),
        within_slice(ROOT_SLICE, toolkit(connection, &root)),
    )
    .await;
    AppRead::Listed(Box::new(assemble(
        root,
        facts,
        name.flatten(),
        toolkit.unwrap_or_default(),
    )))
}

/// One pass over the registry. An application whose bus name or process is
/// gone is reported as skipped; one that does not answer its own calls is
/// still listed, with its process id from the bus daemon and its name from
/// `/proc`.
pub(crate) fn read_registry(bus: &SharedBus, deadline: Deadline) -> Result<Registry, AdapterError> {
    let connection = bus.get(deadline)?;
    within(deadline, async {
        let registry = AtspiElement::new(REGISTRY_SERVICE, REGISTRY_ROOT_PATH, 0);
        let children = list_children(connection.inner(), &registry).await?;
        let reads: Vec<AppRead> = stream::iter(
            children
                .into_iter()
                .map(|(bus, path)| read_app(connection.inner(), bus, path)),
        )
        .buffered(APPS_IN_FLIGHT)
        .collect()
        .await;
        let mut listed = Registry {
            apps: Vec::new(),
            skipped: Vec::new(),
        };
        for read in reads {
            match read {
                AppRead::Listed(app) => listed.apps.push(*app),
                AppRead::Skipped(record) => listed.skipped.push(record),
            }
        }
        Ok(listed)
    })
}

/// One pass over the registry for one process, or for every process when
/// `pid` is `None`.
#[expect(dead_code, reason = "stub the owning lane replaces")]
pub(crate) fn read_registry_for(
    bus: &SharedBus,
    _pid: Option<u32>,
    deadline: Deadline,
) -> Result<Registry, AdapterError> {
    read_registry(bus, deadline)
}
