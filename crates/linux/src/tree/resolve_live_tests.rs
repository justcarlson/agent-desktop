use agent_desktop_core::{
    AdapterError, Deadline, ObservationOps, RefEntry, TreeOptions, WindowFilter, snapshot,
};

use super::live_apps::each_app;
use super::node_read::read_node;
use super::{AtspiElement, LabelOutcome, RawNode, atspi_element, resolve_vocabulary};
use crate::LinuxAdapter;
use crate::system::{SharedBus, window_id, within};

#[derive(Default)]
struct Tally {
    resolved: usize,
    failures: Vec<String>,
}

fn kind(error: &AdapterError) -> String {
    error
        .details
        .as_ref()
        .and_then(|details| details.get("kind"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn entries(result: &snapshot::SnapshotResult) -> Vec<(String, RefEntry)> {
    let value = serde_json::to_value(&result.refmap).expect("a refmap serializes");
    let inner = value["inner"]
        .as_object()
        .expect("a refmap holds its entries");
    let mut found: Vec<(String, RefEntry)> = inner
        .iter()
        .map(|(id, entry)| {
            (
                id.clone(),
                serde_json::from_value(entry.clone()).expect("an entry deserializes"),
            )
        })
        .collect();
    found.sort_by_key(|(id, _)| id.trim_start_matches("@e").parse::<u32>().unwrap_or(0));
    found
}

pub(super) fn independent_read_disagrees(
    bus: &SharedBus,
    handle: &AtspiElement,
    entry: &RefEntry,
) -> Option<String> {
    let deadline = Deadline::after(5_000).expect("the deadline builds");
    let connection = bus.get(deadline).expect("the bus is reachable");
    let fresh = AtspiElement::new(handle.bus_name(), handle.path(), handle.pid());
    let raw =
        within(deadline, read_node(connection.inner(), &fresh)).map_err(|error| error.message);
    disagreement(raw, entry)
}

/// How an independent read of the resolved node differs from the stored
/// entry. A read that failed is a difference: it cannot vouch for the node.
pub(super) fn disagreement(raw: Result<RawNode, String>, entry: &RefEntry) -> Option<String> {
    let raw = match raw {
        Ok(raw) => raw,
        Err(message) => return Some(format!("second read failed: {message}")),
    };
    let vocabulary = resolve_vocabulary(&raw, &LabelOutcome::Unlabelled, false);
    if let Some(role) = vocabulary.role.known()
        && role != &entry.identity.role
    {
        return Some(format!(
            "role {role} against stored {}",
            entry.identity.role
        ));
    }
    if let (Some(name), Some(stored)) = (raw.properties.name.non_blank(), &entry.identity.name)
        && name != stored
    {
        return Some(format!("name {name:?} against stored {stored:?}"));
    }
    if let (Some(stored), Some(extents)) = (entry.geometry.bounds, raw.extents.present())
        && (f64::from(extents.width) != stored.width || f64::from(extents.height) != stored.height)
    {
        return Some(format!("bounds {extents:?} against stored {stored:?}"));
    }
    None
}

pub(super) fn snapshot_entries(
    adapter: &LinuxAdapter,
    frame: &AtspiElement,
    include_bounds: bool,
) -> Result<Vec<(String, RefEntry)>, String> {
    let deadline = Deadline::after(15_000).map_err(|error| error.message)?;
    let id = window_id::encode(frame.bus_name(), frame.path()).ok_or("the frame has no id")?;
    let windows = adapter
        .list_windows(&WindowFilter::default(), deadline)
        .map_err(|error| error.message)?;
    let window = windows
        .into_iter()
        .find(|window| window.id == id)
        .ok_or_else(|| format!("window {id} is not listed"))?;
    let options = TreeOptions {
        include_bounds,
        ..TreeOptions::default()
    };
    let result = snapshot::build(adapter, &options, None, Some(&window.id), deadline)
        .map_err(|error| error.to_string())?;
    Ok(entries(&result))
}

fn resolve_all(
    adapter: &LinuxAdapter,
    bus: &SharedBus,
    frame: &AtspiElement,
    include_bounds: bool,
) -> Result<Tally, String> {
    let mut tally = Tally::default();
    for (ref_id, entry) in snapshot_entries(adapter, frame, include_bounds)? {
        let attempt = Deadline::after(5_000).map_err(|error| error.message)?;
        match adapter.resolve_element_strict(&entry, attempt) {
            Ok(handle) => {
                let element = atspi_element(&handle).map_err(|error| error.message)?;
                match independent_read_disagrees(bus, element, &entry) {
                    None => tally.resolved += 1,
                    Some(difference) => tally.failures.push(format!(
                        "{ref_id} {} {:?} resolved to a node that differs: {difference}",
                        entry.identity.role, entry.identity.name
                    )),
                }
            }
            Err(error) => tally.failures.push(format!(
                "{ref_id} {} {:?} path {:?}: {:?} {} {}",
                entry.identity.role,
                entry.identity.name,
                entry.scope.path,
                error.code,
                kind(&error),
                error.message
            )),
        }
    }
    Ok(tally)
}

fn check(
    label: &str,
    adapter: &LinuxAdapter,
    bus: &SharedBus,
    frame: &AtspiElement,
) -> Result<(), String> {
    let mut failures = Vec::new();
    for include_bounds in [false, true] {
        let tally = resolve_all(adapter, bus, frame, include_bounds)?;
        println!(
            "resolve_live {label} bounds={include_bounds}: resolved={} failed={}",
            tally.resolved,
            tally.failures.len()
        );
        if tally.resolved == 0 {
            failures.push(format!("bounds={include_bounds}: no ref resolved"));
        }
        failures.extend(
            tally
                .failures
                .into_iter()
                .map(|line| format!("bounds={include_bounds}: {line}")),
        );
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(format!("[{label}] {}", failures.join("\n")))
    }
}

#[test]
#[ignore = "drives real desktop applications; run through scripts/private-session.sh"]
fn resolve_live_every_snapshot_ref_resolves_to_the_node_it_was_taken_from() {
    let adapter = LinuxAdapter::new();
    let bus = SharedBus::default();
    let failures: Vec<String> = each_app(&adapter, &bus, check)
        .into_iter()
        .filter_map(|(label, outcome)| outcome.err().map(|error| format!("[{label}] {error}")))
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn an_independent_read_that_failed_is_a_disagreement_and_not_an_agreement() {
    let entry = super::test_support::ref_entry("button");
    let difference = disagreement(Err("bus gone".to_owned()), &entry);
    assert_eq!(difference.as_deref(), Some("second read failed: bus gone"));
}
