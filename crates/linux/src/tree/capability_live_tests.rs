use std::collections::BTreeMap;

use agent_desktop_core::{Deadline, LocatorField};

use super::action_names::{capabilities, capability_of};
use super::capability_rules::carries_checked;
use super::live_apps::each_app;
use super::role_numbers::{COMBO_BOX, DIAL, PUSH_BUTTON, SCROLL_BAR, SLIDER, SPIN_BUTTON};
use super::state_bits::{EXPANDABLE, EXPANDED, HAS_POPUP, StateBits};
use super::walker::walk_tree;
use super::{AtspiElement, ObservedRead, RawNode, WalkLimits, WalkedNode};
use crate::LinuxAdapter;
use crate::system::SharedBus;

const VALUE: &str = "org.a11y.atspi.Value";
const TEXT: &str = "org.a11y.atspi.Text";
const EDITABLE_TEXT: &str = "org.a11y.atspi.EditableText";
const SELECTION: &str = "org.a11y.atspi.Selection";

fn flatten(node: &WalkedNode<ObservedRead>, out: &mut Vec<RawNode>) {
    out.push(node.payload.node.clone());
    for child in &node.children {
        flatten(child, out);
    }
}

fn advertises(node: &RawNode, interface: &str) -> bool {
    node.traits
        .interfaces
        .present()
        .is_some_and(|names| names.iter().any(|name| name == interface))
}

fn names(node: &RawNode) -> &[String] {
    node.actions.present().map_or(&[], Vec::as_slice)
}

fn named(node: &RawNode, wanted: &[&str]) -> bool {
    names(node)
        .iter()
        .any(|name| wanted.iter().any(|w| name.eq_ignore_ascii_case(w)))
}

fn stateful_disclosure(node: &RawNode) -> bool {
    let Some(words) = node.traits.states.present() else {
        return false;
    };
    let bits = StateBits::new(words);
    let verb = named(
        node,
        &[
            "listitem.toggle-expand",
            "listitem.expand",
            "listitem.collapse",
            "click",
            "press",
            "activate",
        ],
    );
    let menu_button = node.traits.role.present() == Some(&PUSH_BUTTON)
        && bits.has(HAS_POPUP)
        && node.properties.child_count.present() == Some(&1);
    (verb && (bits.has(EXPANDABLE) || bits.has(EXPANDED))) || menu_button
}

fn disabled(node: &RawNode) -> bool {
    node.traits
        .states
        .present()
        .is_some_and(|words| StateBits::new(words).is_disabled())
}

fn toggle_only_click(node: &RawNode, role: u32) -> bool {
    carries_checked(role) && named(node, &["toggle", "switch", "uncheck"])
}

fn disabled_click(node: &RawNode, role: u32) -> bool {
    (role == PUSH_BUTTON || carries_checked(role)) && disabled(node)
}

fn popup_dropdown(node: &RawNode, role: u32) -> bool {
    let popup = node
        .traits
        .states
        .present()
        .is_some_and(|words| StateBits::new(words).has(HAS_POPUP));
    role == COMBO_BOX
        && popup
        && !advertises(node, SELECTION)
        && matches!(node.properties.child_count.present(), Some(1 | 2))
}

fn backed(node: &RawNode, token: &str) -> bool {
    let role = node.traits.role.present().copied().unwrap_or_default();
    match token {
        "Click" | "RightClick" | "SetFocus" | "Toggle" => {
            names(node)
                .iter()
                .any(|name| capability_of(name).contains(&token))
                || (token == "Click"
                    && (toggle_only_click(node, role) || disabled_click(node, role)))
        }
        "Select" => {
            named(node, &["select"])
                || popup_dropdown(node, role)
                || (advertises(node, SELECTION)
                    && node
                        .properties
                        .child_count
                        .present()
                        .is_some_and(|n| *n > 0))
        }
        "SetValue" => {
            (matches!(role, SLIDER | SPIN_BUTTON | DIAL | SCROLL_BAR) && advertises(node, VALUE))
                || advertises(node, EDITABLE_TEXT)
        }
        "TypeText" => advertises(node, EDITABLE_TEXT) && advertises(node, TEXT),
        "Expand" | "Collapse" => stateful_disclosure(node),
        _ => false,
    }
}

fn check(
    label: &str,
    _adapter: &LinuxAdapter,
    bus: &SharedBus,
    frame: &AtspiElement,
) -> Result<(), String> {
    let deadline = Deadline::after(15_000).map_err(|error| error.message)?;
    let walk =
        walk_tree(bus, frame, &WalkLimits::default(), deadline).map_err(|error| error.message)?;
    let mut nodes = Vec::new();
    flatten(&walk.root, &mut nodes);
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut problems = Vec::new();
    for node in &nodes {
        let LocatorField::Known(tokens) = capabilities(node) else {
            if node.failed_reads() == 0 {
                problems.push(format!(
                    "unknown capabilities on a fully read node {:?}",
                    node.properties.name
                ));
            }
            continue;
        };
        for token in tokens {
            *counts.entry(token.clone()).or_default() += 1;
            if !backed(node, &token) {
                problems.push(format!(
                    "{token} on role {:?} {:?} without its interface or action: actions {:?}",
                    node.traits.role, node.properties.name, node.actions
                ));
            }
        }
    }
    let disclosures = nodes
        .iter()
        .filter(|node| {
            matches!(capabilities(node), LocatorField::Known(tokens) if tokens.iter().any(|t| t == "Expand"))
        })
        .count();
    println!(
        "capability_live {label}: nodes={} tokens={counts:?} expand_nodes={disclosures} problems={}",
        nodes.len(),
        problems.len()
    );
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("\n"))
    }
}

#[test]
#[ignore = "drives real desktop applications; run through scripts/private-session.sh"]
fn capability_live_every_token_is_backed_by_an_interface_or_an_action() {
    let adapter = LinuxAdapter::new();
    let bus = SharedBus::default();
    let failures: Vec<String> = each_app(&adapter, &bus, check)
        .into_iter()
        .filter_map(|(label, outcome)| outcome.err().map(|error| format!("[{label}] {error}")))
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
