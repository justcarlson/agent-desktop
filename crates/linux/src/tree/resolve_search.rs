use agent_desktop_core::{BoundsSpace, LocatorField, RefEntry};

use super::candidate::Candidate;
use super::projection::project_node;
use super::resolve_match::{
    Admission, accepts_landing, admit_node, provisional_geometry_candidate,
};
use super::role_numbers::is_web_document;
use super::searched::Searched;
use super::{ObservedRead, Walk, WalkLimits, WalkedNode};

fn childless_web_region(node: &WalkedNode<ObservedRead>, is_root: bool, web_process: bool) -> bool {
    let role = node.payload.node.traits.role.present().copied();
    let opens_web = is_web_document(role) || (is_root && web_process);
    opens_web && node.children.is_empty() && !node.truncated
}

fn visit(
    node: &WalkedNode<ObservedRead>,
    entry: &RefEntry,
    depth: usize,
    web_process: bool,
    found: &mut Searched,
) {
    let is_root = depth == 0;
    let limits = WalkLimits::default();
    let mut evidence = project_node(&node.payload, node.in_web_document, &limits);
    if entry.source.source_surface == agent_desktop_core::SnapshotSurface::Menu {
        evidence.identifiers = evidence
            .identifiers
            .with_retained_object(node.payload.node.element.object_identity());
    }
    let admission = if provisional_geometry_candidate(entry) && accepts_landing(entry, &evidence) {
        Admission::Collect
    } else {
        admit_node(entry, &evidence)
    };
    match admission {
        Admission::Collect => found.candidates.push(Candidate {
            element: node.payload.node.element.clone(),
            bounds: match evidence.ref_evidence.window_bounds {
                LocatorField::Known(rect) => Some(rect),
                LocatorField::Absent | LocatorField::Unknown => {
                    match evidence.ref_evidence.bounds {
                        LocatorField::Known(rect) => Some(rect),
                        LocatorField::Absent | LocatorField::Unknown => None,
                    }
                }
            },
            bounds_space: match evidence.ref_evidence.window_bounds {
                LocatorField::Known(_) => BoundsSpace::Window,
                LocatorField::Absent | LocatorField::Unknown => BoundsSpace::Screen,
            },
            depth,
        }),
        Admission::Unread => found.unread_region = true,
        Admission::Reject => {}
    }
    found.unread_region |= childless_web_region(node, is_root, web_process);
    for child in &node.children {
        visit(child, entry, depth + 1, web_process, found);
    }
}

/// Evaluates every node of a finished walk against the stored evidence. A walk
/// that was cut short, a node whose evidence could not settle the question, and
/// a childless web document (a cold Chromium tree lists no children until a
/// read other than `GetChildren`, L17-17) each leave a region unread, so no
/// negative verdict follows from them.
pub(crate) fn evaluate(walk: &Walk<ObservedRead>, entry: &RefEntry, web_process: bool) -> Searched {
    let mut found = Searched {
        candidates: Vec::new(),
        unread_region: !walk.complete,
    };
    visit(&walk.root, entry, 0, web_process, &mut found);
    found
}

#[cfg(test)]
#[path = "resolve_search_tests.rs"]
mod tests;
