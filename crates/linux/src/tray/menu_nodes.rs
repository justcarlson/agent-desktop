use agent_desktop_core::{LocatorEvidence, ObservedSubtree, state};

use super::TrayTarget;
use super::capability;
use super::menu_layout::{Layout, LayoutEntry};
use super::node_evidence::{ENTRY_ROLE, SEPARATOR_ROLE, fresh};

#[cfg(test)]
pub(crate) fn child_count(entries: &[LayoutEntry]) -> usize {
    entries
        .iter()
        .map(|entry| 1 + child_count(&entry.children))
        .sum()
}

pub(crate) fn states_of(entry: &LayoutEntry) -> Vec<String> {
    let mut states = vec![state::OFFSCREEN.to_owned()];
    if !entry.enabled {
        states.push(state::DISABLED.to_owned());
    }
    if entry.checked == Some(true) {
        states.push(state::CHECKED.to_owned());
    }
    if entry.submenu {
        states.push(state::HASPOPUP.to_owned());
    }
    states
}

pub(crate) fn role_of(entry: &LayoutEntry) -> &'static str {
    if entry.separator {
        SEPARATOR_ROLE
    } else {
        ENTRY_ROLE
    }
}

pub(crate) fn entry_target(
    item: &TrayTarget,
    menu_path: &str,
    revision: u32,
    labels: Vec<String>,
    id: i32,
) -> TrayTarget {
    TrayTarget {
        owner: item.owner.clone(),
        object_path: item.object_path.clone(),
        menu_path: Some(menu_path.to_owned()),
        entry_id: Some(id),
        revision: Some(revision),
        label_path: labels,
    }
}

fn evidence_of(entry: &LayoutEntry, target: &TrayTarget) -> LocatorEvidence {
    fresh(
        role_of(entry),
        &entry.label,
        target,
        states_of(entry),
        capability::entry_actions(entry),
    )
}

struct Context<'a> {
    item: &'a TrayTarget,
    menu_path: &'a str,
    revision: u32,
}

fn build(
    context: &Context<'_>,
    entries: &[LayoutEntry],
    labels: &[String],
) -> Vec<ObservedSubtree> {
    entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let mut path = labels.to_vec();
            path.push(entry.label.clone());
            let target = entry_target(
                context.item,
                context.menu_path,
                context.revision,
                path.clone(),
                entry.id,
            );
            let children = build(context, &entry.children, &path);
            ObservedSubtree::new(evidence_of(entry, &target), children, true, None)
                .with_source_child_index(index)
        })
        .collect()
}

/// The entries of a tray item's menu as subtrees, each carrying the persisted
/// identity that names it: owner, object, menu, entry id and label path.
pub(crate) fn subtrees(
    item: &TrayTarget,
    menu_path: &str,
    layout: &Layout,
) -> Vec<ObservedSubtree> {
    let context = Context {
        item,
        menu_path,
        revision: layout.revision,
    };
    build(&context, &layout.root.children, &[])
}

#[cfg(test)]
#[path = "menu_nodes_tests.rs"]
mod tests;
