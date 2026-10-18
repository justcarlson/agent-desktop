use std::collections::{HashMap, HashSet};

use agent_desktop_core::AdapterError;
use futures_util::future::join_all;
use futures_util::stream::{self, StreamExt};
use zbus::Connection;

use super::node_read::read_node;
use super::{AtspiElement, RawNode};
use crate::system::classify;

const BUS_DAEMON: &str = "org.freedesktop.DBus";
const BUS_DAEMON_PATH: &str = "/org/freedesktop/DBus";
const NODES_IN_FLIGHT: usize = 8;

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

async fn learn_pids(
    connection: &Connection,
    pids: &mut HashMap<String, u32>,
    reads: &[RawNode],
) -> Result<(), AdapterError> {
    let unseen: HashSet<&str> = reads
        .iter()
        .filter_map(|node| node.children.present())
        .flatten()
        .map(|(bus, _)| bus.as_str())
        .filter(|bus| !pids.contains_key(*bus))
        .collect();
    let lookups = unseen
        .into_iter()
        .map(|bus| process_id(connection, bus.to_owned()));
    for (bus, pid) in join_all(lookups).await {
        pids.insert(bus, pid?);
    }
    Ok(())
}

fn child_elements(node: &RawNode, pids: &HashMap<String, u32>) -> Vec<AtspiElement> {
    node.children
        .present()
        .into_iter()
        .flatten()
        .filter_map(|(bus, path)| {
            pids.get(bus)
                .map(|pid| AtspiElement::new(bus.as_str(), path.as_str(), *pid))
        })
        .collect()
}

/// Reads one level of the tree with at most `NODES_IN_FLIGHT` nodes (about
/// sixty calls) outstanding at a time. A node that is gone fails the walk
/// when it is the root and when its whole application is gone; below the
/// root it is kept as an unreadable node.
pub(super) fn expand_level(
    connection: &Connection,
    pids: &mut HashMap<String, u32>,
    level: &[AtspiElement],
    at_root: bool,
) -> Result<Vec<(RawNode, Vec<AtspiElement>)>, AdapterError> {
    zbus::block_on(async {
        let reads: Vec<Result<RawNode, AdapterError>> =
            stream::iter(level.iter().map(|element| read_node(connection, element)))
                .buffered(NODES_IN_FLIGHT)
                .collect()
                .await;
        let mut nodes = Vec::with_capacity(reads.len());
        for (element, read) in level.iter().zip(reads) {
            match read {
                Ok(node) => nodes.push(node),
                Err(error)
                    if at_root || error.code == agent_desktop_core::ErrorCode::AppNotFound =>
                {
                    return Err(error);
                }
                Err(error) => nodes.push(RawNode::unreadable(element.clone(), &error.message)),
            }
        }
        learn_pids(connection, pids, &nodes).await?;
        Ok(nodes
            .into_iter()
            .map(|node| {
                let children = child_elements(&node, pids);
                (node, children)
            })
            .collect())
    })
}
