use std::collections::{HashMap, HashSet};

use agent_desktop_core::{AdapterError, Deadline, ErrorCode};
use futures_util::future::join_all;
use futures_util::stream::{self, StreamExt};
use zbus::Connection;

use super::node_read::read_node;
use super::{AtspiElement, Expansion, RawNode};
use crate::system::{classify, within};

const BUS_DAEMON: &str = "org.freedesktop.DBus";
const BUS_DAEMON_PATH: &str = "/org/freedesktop/DBus";
const NODES_IN_FLIGHT: usize = 8;

/// Stands in for the process id of a bus name whose lookup failed. Process
/// id 0 is never an application, so a child carrying it is recorded as
/// unreadable instead of being read.
pub(super) const UNKNOWN_PID: u32 = 0;
const UNIDENTIFIED: &str = "The owning process of the bus name could not be identified";

async fn process_id(
    connection: &Connection,
    bus_name: String,
) -> (String, Result<u32, AdapterError>) {
    let pid = connection
        .call_method(
            Some(BUS_DAEMON),
            BUS_DAEMON_PATH,
            Some(BUS_DAEMON),
            "GetConnectionUnixProcessID",
            &(bus_name.as_str(),),
        )
        .await
        .and_then(|reply| reply.body().deserialize::<u32>())
        .map_err(|error| classify(&error));
    (bus_name, pid)
}

async fn learn_pids<'a>(
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
    record_pids(pids, join_all(lookups).await)
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
    node.children
        .present()
        .into_iter()
        .flatten()
        .map(|(bus, path)| {
            let pid = pids.get(bus).copied().unwrap_or(UNKNOWN_PID);
            AtspiElement::new(bus.as_str(), path.as_str(), pid)
        })
        .collect()
}

/// Reads one level of the tree with at most `NODES_IN_FLIGHT` nodes (about
/// sixty calls) outstanding at a time. Only the root being gone fails the
/// walk. Below the root a node that is gone, or whose process id could not be
/// found, is kept as an unreadable node marked gone.
pub(super) fn expand_level(
    connection: &Connection,
    pids: &mut HashMap<String, u32>,
    level: &[AtspiElement],
    at_root: bool,
    deadline: Deadline,
) -> Result<Vec<Expansion<RawNode, AtspiElement>>, AdapterError> {
    within(deadline, async {
        let reads: Vec<Result<RawNode, AdapterError>> =
            stream::iter(level.iter().map(|element| async move {
                if !at_root && element.pid() == UNKNOWN_PID {
                    Err(AdapterError::new(ErrorCode::ElementNotFound, UNIDENTIFIED))
                } else {
                    read_node(connection, element).await
                }
            }))
            .buffered(NODES_IN_FLIGHT)
            .collect()
            .await;
        let mut nodes = Vec::with_capacity(reads.len());
        for (element, read) in level.iter().zip(reads) {
            match read {
                Ok(node) => nodes.push((node, false)),
                Err(error) if at_root => return Err(error),
                Err(error) => {
                    nodes.push((RawNode::unreadable(element.clone(), &error.message), true))
                }
            }
        }
        learn_pids(connection, pids, nodes.iter().map(|(node, _)| node)).await?;
        Ok(nodes
            .into_iter()
            .map(|(node, gone)| {
                let children = child_elements(&node, pids);
                Expansion {
                    children_unread: node.children.is_failed(),
                    payload: node,
                    children,
                    gone,
                }
            })
            .collect())
    })
}
