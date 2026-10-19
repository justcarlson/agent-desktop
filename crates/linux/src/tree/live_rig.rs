use std::time::{Duration, Instant};

use agent_desktop_core::{AdapterError, Deadline, NativeHandle, ObservationOps, RefEntry};

use super::live_launch::Launched;
use super::live_tree::pid_frame;
use super::node_read::{call, read_node};
use super::resolve_live_tests::snapshot_entries;
use super::walker_live_tests::{Settled, settle};
use super::{AtspiElement, RawNode, atspi_element};
use crate::LinuxAdapter;
use crate::system::{SharedBus, within};

const ACTION_INTERFACE: &str = "org.a11y.atspi.Action";

/// The adapter and bus a live test drives, with the processes it launched.
pub(super) struct Rig {
    pub(super) adapter: LinuxAdapter,
    pub(super) bus: SharedBus,
    pub(super) launched: Launched,
}

pub(super) fn rig() -> Rig {
    Rig {
        adapter: LinuxAdapter::new(),
        bus: SharedBus::default(),
        launched: Launched::new(),
    }
}

pub(super) fn deadline(ms: u64) -> Deadline {
    Deadline::after(ms).expect("the deadline builds")
}

pub(super) fn kind(error: &AdapterError) -> String {
    error
        .details
        .as_ref()
        .and_then(|details| details.get("kind"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

impl Rig {
    pub(super) fn settled(&self, pid: u32, min_nodes: usize) -> Settled {
        let connection = self
            .bus
            .get(Deadline::standard().expect("a standard deadline builds"))
            .expect("the accessibility bus is reachable");
        settle(&self.bus, min_nodes, || pid_frame(&connection, pid)).expect("the app settles")
    }

    pub(super) fn entries(
        &self,
        frame: &AtspiElement,
        include_bounds: bool,
    ) -> Vec<(String, RefEntry)> {
        snapshot_entries(&self.adapter, frame, include_bounds).expect("a snapshot")
    }

    pub(super) fn named(
        &self,
        frame: &AtspiElement,
        include_bounds: bool,
        name: &str,
    ) -> Vec<RefEntry> {
        self.entries(frame, include_bounds)
            .into_iter()
            .map(|(_, entry)| entry)
            .filter(|entry| entry.identity.name.as_deref() == Some(name))
            .collect()
    }

    pub(super) fn resolve(&self, entry: &RefEntry) -> Result<NativeHandle, AdapterError> {
        self.adapter.resolve_element_strict(entry, deadline(5_000))
    }

    pub(super) fn timed_resolve(
        &self,
        entry: &RefEntry,
    ) -> (Result<NativeHandle, AdapterError>, Duration) {
        let started = Instant::now();
        let outcome = self.resolve(entry);
        (outcome, started.elapsed())
    }

    pub(super) fn raw(&self, handle: &NativeHandle) -> Result<RawNode, AdapterError> {
        let element = atspi_element(handle)?;
        let budget = deadline(5_000);
        let connection = self.bus.get(budget)?;
        let fresh = AtspiElement::new(element.bus_name(), element.path(), element.pid());
        within(budget, read_node(connection.inner(), &fresh))
    }

    pub(super) fn do_action(&self, handle: &NativeHandle, index: i32) -> Result<(), AdapterError> {
        self.invoke(handle, ACTION_INTERFACE, "DoAction", index)
    }

    fn invoke(
        &self,
        handle: &NativeHandle,
        interface: &str,
        method: &str,
        index: i32,
    ) -> Result<(), AdapterError> {
        let element = atspi_element(handle)?;
        let budget = deadline(5_000);
        let connection = self.bus.get(budget)?;
        within(budget, async {
            call(connection.inner(), element, interface, method, &(index,))
                .await
                .map(|_| ())
                .map_err(|error| AdapterError::internal(error.to_string()))
        })
    }
}

pub(super) fn width_of(raw: &RawNode) -> f64 {
    f64::from(raw.extents.present().expect("the node has extents").width)
}

pub(super) fn wait_until(limit: Duration, mut condition: impl FnMut() -> bool) -> bool {
    let started = Instant::now();
    while started.elapsed() < limit {
        if condition() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    false
}

pub(super) fn signal(pid: u32, name: &str) {
    let status = std::process::Command::new("kill")
        .args([&format!("-{name}"), &pid.to_string()])
        .status()
        .expect("kill runs");
    assert!(status.success(), "kill -{name} {pid} failed");
}

pub(super) fn refused<T>(outcome: Result<T, AdapterError>, what: &str) -> AdapterError {
    match outcome {
        Ok(_) => panic!("{what}"),
        Err(error) => error,
    }
}
