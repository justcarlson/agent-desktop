use std::fmt::Write as _;

use zbus::blocking::Connection;

use super::{AtspiElement, Field, RawNode, WalkedNode};
use crate::system::{ACCESSIBLE_INTERFACE, REGISTRY_ROOT_PATH, REGISTRY_SERVICE};

const BUS_DAEMON: &str = "org.freedesktop.DBus";

pub(super) fn children_of(connection: &Connection, bus: &str, path: &str) -> Vec<(String, String)> {
    let Ok(reply) = connection.call_method(
        Some(bus),
        path,
        Some(ACCESSIBLE_INTERFACE),
        "GetChildren",
        &(),
    ) else {
        return Vec::new();
    };
    reply
        .body()
        .deserialize::<Vec<(String, zbus::zvariant::OwnedObjectPath)>>()
        .map(|entries| {
            entries
                .into_iter()
                .map(|(bus, path)| (bus, path.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn pid_of(connection: &Connection, bus: &str) -> Option<u32> {
    connection
        .call_method(
            Some(BUS_DAEMON),
            "/org/freedesktop/DBus",
            Some(BUS_DAEMON),
            "GetConnectionUnixProcessID",
            &(bus,),
        )
        .ok()?
        .body()
        .deserialize::<u32>()
        .ok()
}

pub(super) fn frames_where(
    connection: &Connection,
    wanted: impl Fn(u32) -> bool,
) -> Vec<AtspiElement> {
    let mut frames = Vec::new();
    for (app_bus, _) in children_of(connection, REGISTRY_SERVICE, REGISTRY_ROOT_PATH) {
        let Some(pid) = pid_of(connection, &app_bus) else {
            continue;
        };
        if !wanted(pid) {
            continue;
        }
        for (frame_bus, frame_path) in children_of(connection, &app_bus, REGISTRY_ROOT_PATH) {
            frames.push(AtspiElement::new(frame_bus, frame_path, pid));
        }
    }
    frames
}

pub(super) fn count_failed(node: &WalkedNode<RawNode>) -> usize {
    node.payload.failed_reads() + node.children.iter().map(count_failed).sum::<usize>()
}

pub(super) fn collect_failures(node: &WalkedNode<RawNode>, found: &mut Vec<String>) {
    for (field, detail) in node.payload.failures() {
        let role = node
            .payload
            .traits
            .role_name
            .present()
            .map_or("?", String::as_str);
        found.push(format!("{role}.{field}: {detail}"));
    }
    for child in &node.children {
        collect_failures(child, found);
    }
}

pub(super) fn failure_kinds(node: &WalkedNode<RawNode>) -> Vec<String> {
    let mut found = Vec::new();
    collect_failures(node, &mut found);
    found.sort();
    found.dedup();
    found
}

pub(super) fn dump(node: &WalkedNode<RawNode>, depth: usize, out: &mut String) {
    let payload = &node.payload;
    let role = payload
        .traits
        .role_name
        .present()
        .map_or("?", String::as_str);
    let name = payload.properties.name.present().map_or("", String::as_str);
    let _ = writeln!(
        out,
        "{}{role} {name:?} failed={} truncated={}",
        "  ".repeat(depth),
        payload.failed_reads(),
        node.truncated
    );
    for child in &node.children {
        dump(child, depth + 1, out);
    }
}

pub(super) fn find_node<'a>(
    node: &'a WalkedNode<RawNode>,
    wanted: &impl Fn(&RawNode) -> bool,
) -> Option<&'a RawNode> {
    if wanted(&node.payload) {
        return Some(&node.payload);
    }
    node.children
        .iter()
        .find_map(|child| find_node(child, wanted))
}

pub(super) fn action_summary(node: &WalkedNode<RawNode>) -> String {
    let mut found = Vec::new();
    collect_actions(node, &mut found);
    format!("{} nodes with an action field: {found:?}", found.len())
}

pub(super) fn collect_actions(node: &WalkedNode<RawNode>, found: &mut Vec<String>) {
    if !matches!(node.payload.actions, Field::Absent) {
        found.push(format!("{:?}", node.payload.actions));
    }
    for child in &node.children {
        collect_actions(child, found);
    }
}

pub(super) fn pid_frame(connection: &Connection, pid: u32) -> Option<AtspiElement> {
    frames_where(connection, |candidate| candidate == pid)
        .into_iter()
        .next()
}
