use agent_desktop_core::{Deadline, ObservationOps, RefEntry, TreeOptions, WindowFilter, snapshot};

use super::node_read::read_node;
use super::{AtspiElement, LabelOutcome, RawNode, resolve_vocabulary};
use crate::LinuxAdapter;
use crate::system::{SharedBus, window_id, within};

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

#[test]
fn an_independent_read_that_failed_is_a_disagreement_and_not_an_agreement() {
    let entry = super::test_support::ref_entry("button");
    let difference = disagreement(Err("bus gone".to_owned()), &entry);
    assert_eq!(difference.as_deref(), Some("second read failed: bus gone"));
}
