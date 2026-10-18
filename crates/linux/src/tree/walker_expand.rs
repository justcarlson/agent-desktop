use std::collections::{HashMap, HashSet};

use agent_desktop_core::{AdapterError, Deadline, ErrorCode};
use futures_util::future::join;
use futures_util::stream::{self, StreamExt};
use zbus::Connection;

use super::absence::Liveness;
use super::ancestry;
use super::bounded::collect_bounded;
use super::label_read::read_label_if_needed;
use super::node_read::read_node_with_liveness;
use super::value_read::read_value;
use super::{AtspiElement, Expansion, ObservedRead, RawNode, WalkLimits};
use crate::system::{process_id, within};

const NODES_IN_FLIGHT: usize = 8;
const PID_LOOKUPS_IN_FLIGHT: usize = 64;

/// Stands in for the process id of a bus name whose lookup failed. Process
/// id 0 is never an application, so a child carrying it is recorded as
/// unreadable instead of being read.
pub(super) const UNKNOWN_PID: u32 = 0;
const UNIDENTIFIED: &str = "The owning process of the bus name could not be identified";

pub(super) async fn learn_pids<'a>(
    connection: &Connection,
    pids: &mut HashMap<String, u32>,
    reads: impl Iterator<Item = &'a RawNode>,
) -> Result<(), AdapterError> {
    let unseen: HashSet<&str> = reads
        .filter_map(|node| node.children.present())
        .flatten()
        .map(|(bus, _)| bus.as_str())
        .filter(|bus| !pids.contains_key(*bus))
        .collect();
    let lookups = unseen
        .into_iter()
        .map(|bus| process_id(connection, bus.to_owned()));
    record_pids(pids, collect_bounded(PID_LOOKUPS_IN_FLIGHT, lookups).await)
}

/// Stores each resolved process id. A lookup that failed because the bus
/// name is gone or unanswerable leaves that name unrecorded, so its children
/// are kept as unreadable nodes; only an exhausted budget fails the level.
pub(super) fn record_pids(
    pids: &mut HashMap<String, u32>,
    lookups: Vec<(String, Result<u32, AdapterError>)>,
) -> Result<(), AdapterError> {
    for (bus, pid) in lookups {
        match pid {
            Ok(pid) => {
                pids.insert(bus, pid);
            }
            Err(error) if error.code == ErrorCode::Timeout => return Err(error),
            Err(_) => {}
        }
    }
    Ok(())
}

pub(super) fn child_elements(node: &RawNode, pids: &HashMap<String, u32>) -> Vec<AtspiElement> {
    let ancestry = ancestry::child(&node.element, node.traits.role.present().copied());
    node.children
        .present()
        .into_iter()
        .flatten()
        .map(|(bus, path)| {
            let pid = pids.get(bus).copied().unwrap_or(UNKNOWN_PID);
            let mut element = AtspiElement::new(bus.as_str(), path.as_str(), pid);
            element.set_ancestry(ancestry.clone());
            element
        })
        .collect()
}

/// Reads the levels of one walk: the shared connection, the process ids
/// learned so far, and what a node read must carry.
pub(super) struct LevelReader<'a> {
    connection: &'a Connection,
    pids: HashMap<String, u32>,
    max_field_bytes: usize,
    deadline: Deadline,
    at_root: bool,
}

impl<'a> LevelReader<'a> {
    pub(super) fn new(
        connection: &'a Connection,
        root: &AtspiElement,
        limits: &WalkLimits,
        deadline: Deadline,
    ) -> Self {
        Self {
            connection,
            pids: HashMap::from([(root.bus_name().to_owned(), root.pid())]),
            max_field_bytes: limits.max_field_bytes,
            deadline,
            at_root: true,
        }
    }

    /// Reads one level of the tree with at most `NODES_IN_FLIGHT` nodes (about
    /// sixty calls) outstanding at a time. Only the root being gone fails the
    /// walk. Below the root a node that is gone, or whose process id could not
    /// be found, is kept as an unreadable node marked gone.
    pub(super) fn expand(
        &mut self,
        level: &[AtspiElement],
    ) -> Result<Vec<Expansion<ObservedRead, AtspiElement>>, AdapterError> {
        let at_root = std::mem::replace(&mut self.at_root, false);
        let (connection, max_bytes) = (self.connection, self.max_field_bytes);
        let pids = &mut self.pids;
        within(self.deadline, async {
            let reads: Vec<Result<ObservedRead, AdapterError>> =
                stream::iter(level.iter().map(|element| async move {
                    if !at_root && element.pid() == UNKNOWN_PID {
                        Err(AdapterError::new(ErrorCode::ElementNotFound, UNIDENTIFIED))
                    } else {
                        read_observed(connection, element, max_bytes).await
                    }
                }))
                .buffered(NODES_IN_FLIGHT)
                .collect()
                .await;
            let mut nodes = Vec::with_capacity(reads.len());
            for (element, read) in level.iter().zip(reads) {
                match read {
                    Ok(read) => nodes.push((read, false)),
                    Err(error) if at_root => return Err(error),
                    Err(error) => nodes.push((
                        ObservedRead::unreadable(element.clone(), &error.message),
                        true,
                    )),
                }
            }
            learn_pids(connection, pids, nodes.iter().map(|(read, _)| &read.node)).await?;
            Ok(nodes
                .into_iter()
                .map(|(read, gone)| {
                    let children = child_elements(&read.node, pids);
                    Expansion {
                        children_unread: read.node.children.is_failed(),
                        payload: read,
                        children,
                        gone,
                    }
                })
                .collect())
        })
    }
}

pub(super) async fn read_observed(
    connection: &Connection,
    element: &AtspiElement,
    max_field_bytes: usize,
) -> Result<ObservedRead, AdapterError> {
    read_observed_with_liveness(connection, element, max_field_bytes)
        .await
        .map(|(read, _)| read)
}

/// The walk's own per-node read, with the liveness the node's `GetRole` showed.
pub(super) async fn read_observed_with_liveness(
    connection: &Connection,
    element: &AtspiElement,
    max_field_bytes: usize,
) -> Result<(ObservedRead, Liveness), AdapterError> {
    let (node, liveness) = read_node_with_liveness(connection, element).await?;
    let (label, value) = join(
        read_label_if_needed(connection, &node),
        read_value(connection, &node, max_field_bytes),
    )
    .await;
    Ok((ObservedRead { node, label, value }, liveness))
}
