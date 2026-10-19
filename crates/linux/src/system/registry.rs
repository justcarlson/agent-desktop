use std::time::Duration;

use agent_desktop_core::{AdapterError, Deadline};
use futures_util::future::join;
use futures_util::stream::{self, StreamExt};
use serde_json::Value;
use zbus::Connection;
use zbus::zvariant::OwnedValue;

use super::bus_slice::within_slice;
use super::process_facts::{self, ProcessFacts};
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
    Excluded,
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

pub(super) trait AppReads {
    fn facts(&self, pid: u32) -> Option<ProcessFacts>;
    async fn name(&self, root: &AtspiElement) -> Option<Option<String>>;
    async fn toolkit(&self, root: &AtspiElement) -> ToolkitId;
}

struct BusReads<'a>(&'a Connection);

impl AppReads for BusReads<'_> {
    fn facts(&self, pid: u32) -> Option<ProcessFacts> {
        process_facts::read(pid)
    }

    async fn name(&self, root: &AtspiElement) -> Option<Option<String>> {
        within_slice(ROOT_SLICE, root_name(self.0, root))
            .await
            .flatten()
    }

    async fn toolkit(&self, root: &AtspiElement) -> ToolkitId {
        within_slice(ROOT_SLICE, toolkit(self.0, root))
            .await
            .unwrap_or_default()
    }
}

pub(super) async fn read_known(
    reads: &impl AppReads,
    root: AtspiElement,
    only: Option<u32>,
) -> AppRead {
    let pid = root.pid();
    if only.is_some_and(|wanted| wanted != pid) {
        return AppRead::Excluded;
    }
    let Some(facts) = reads.facts(pid) else {
        return AppRead::Skipped(skipped(
            root.bus_name(),
            Some(pid),
            "the process has exited",
        ));
    };
    let (name, toolkit) = join(reads.name(&root), reads.toolkit(&root)).await;
    AppRead::Listed(Box::new(assemble(root, facts, name, toolkit)))
}

async fn read_app(
    connection: &Connection,
    bus: String,
    path: String,
    only: Option<u32>,
) -> AppRead {
    let pid = match process_id(connection, bus.clone()).await.1 {
        Ok(pid) => pid,
        Err(error) => return AppRead::Skipped(skipped(&bus, None, &error.message)),
    };
    read_known(
        &BusReads(connection),
        AtspiElement::new(bus, path, pid),
        only,
    )
    .await
}

/// One pass over the registry. An application whose bus name or process is
/// gone is reported as skipped; one that does not answer its own calls is
/// still listed, with its process id from the bus daemon and its name from
/// `/proc`.
pub(crate) fn read_registry(bus: &SharedBus, deadline: Deadline) -> Result<Registry, AdapterError> {
    read_registry_for(bus, None, deadline)
}

/// One pass over the registry for one process, or for every process when
/// `pid` is `None`. Another process's application is left out of the result
/// and its facts, name and toolkit are never read.
pub(crate) fn read_registry_for(
    bus: &SharedBus,
    pid: Option<u32>,
    deadline: Deadline,
) -> Result<Registry, AdapterError> {
    let connection = bus.get(deadline)?;
    within(deadline, async {
        let registry = AtspiElement::new(REGISTRY_SERVICE, REGISTRY_ROOT_PATH, 0);
        let children = list_children(connection.inner(), &registry).await?;
        let reads: Vec<AppRead> = stream::iter(
            children
                .into_iter()
                .map(|(bus, path)| read_app(connection.inner(), bus, path, pid)),
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
                AppRead::Excluded => {}
            }
        }
        Ok(listed)
    })
}

#[cfg(test)]
#[path = "registry_tests.rs"]
mod tests;
