use agent_desktop_core::{
    LocatorEvidence, LocatorField, LocatorRefEvidence, LocatorStats, ObservedSubtree, Rect,
};

use super::action_names::capabilities;
use super::value_read::truncate_to_boundary;
use super::vocabulary::resolve_vocabulary;
use super::{Extents, Field, ObservedRead, Walk, WalkLimits, WalkedNode};

fn bounds_of(extents: &Field<Extents>) -> LocatorField<Rect> {
    match extents {
        Field::Present(extents) => LocatorField::Known(Rect {
            x: f64::from(extents.x),
            y: f64::from(extents.y),
            width: f64::from(extents.width),
            height: f64::from(extents.height),
        }),
        Field::Absent => LocatorField::Absent,
        Field::Failed(_) => LocatorField::Unknown,
    }
}

fn value_of(read: &ObservedRead) -> LocatorField<String> {
    match &read.value.value {
        Field::Present(text) => LocatorField::Known(text.clone()),
        Field::Absent => LocatorField::Absent,
        Field::Failed(_) => LocatorField::Unknown,
    }
}

struct Projector<'a> {
    limits: &'a WalkLimits,
    stats: LocatorStats,
}

impl Projector<'_> {
    fn capped(&mut self, field: LocatorField<String>) -> LocatorField<String> {
        match field {
            LocatorField::Known(text) => {
                let (kept, cut) = truncate_to_boundary(text, self.limits.max_field_bytes);
                self.stats.traversal.limits.text_hits += u64::from(cut);
                LocatorField::Known(kept)
            }
            other => other,
        }
    }

    fn failures_of(read: &ObservedRead) -> u64 {
        let label = matches!(read.label, super::LabelOutcome::Failed);
        (read.node.failed_reads() + usize::from(read.value.value.is_failed()) + usize::from(label))
            as u64
    }

    fn evidence(&mut self, node: &WalkedNode<ObservedRead>) -> LocatorEvidence {
        let read = &node.payload;
        let vocabulary = resolve_vocabulary(&read.node, &read.label, node.in_web_document);
        let truncated_value = u64::from(read.value.truncated);
        self.stats.traversal.limits.text_hits += truncated_value;
        let identifiers = vocabulary.identifiers.identifiers().len() as u64;
        self.stats.identifiers.values_observed += identifiers;
        self.stats.identifiers.nodes_with_identifiers += u64::from(identifiers > 0);
        self.stats.identifiers.nodes_with_multiple_identifiers += u64::from(identifiers > 1);
        self.stats.reads.health.native_read_failures += Self::failures_of(read);
        self.stats.reads.counts.action_reads += u64::from(read.node.actions.present().is_some());
        LocatorEvidence {
            role: vocabulary.role,
            name: self.capped(vocabulary.name),
            description: self.capped(vocabulary.description),
            value: value_of(read),
            identifiers: vocabulary.identifiers,
            states: vocabulary.states,
            ref_evidence: LocatorRefEvidence {
                bounds: bounds_of(&read.node.extents),
                available_actions: capabilities(
                    &read.node.actions,
                    &read.node.traits.role,
                    &read.node.traits.states,
                ),
                descriptors: vocabulary.descriptor,
            },
        }
    }

    fn subtree(&mut self, node: WalkedNode<ObservedRead>) -> ObservedSubtree {
        let evidence = self.evidence(&node);
        let WalkedNode {
            children,
            truncated,
            child_count,
            source_index,
            ..
        } = node;
        let children = children
            .into_iter()
            .map(|child| self.subtree(child))
            .collect();
        ObservedSubtree::new(evidence, children, !truncated, child_count)
            .with_source_child_index(source_index)
    }
}

/// Turns a finished walk into core's observed subtree and the statistics of
/// the walk. Every field is known, absent or unknown exactly as it was read:
/// a read that failed is unknown, never an empty value.
pub(crate) fn project(
    walk: Walk<ObservedRead>,
    limits: &WalkLimits,
) -> (ObservedSubtree, LocatorStats) {
    let mut projector = Projector {
        limits,
        stats: LocatorStats::default(),
    };
    let counts = walk.counts;
    projector.stats.traversal.nodes_visited = walk.node_count as u64;
    projector.stats.traversal.max_raw_depth = u8::try_from(walk.max_depth).unwrap_or(u8::MAX);
    projector.stats.traversal.max_logical_depth =
        u8::try_from(counts.max_logical_depth).unwrap_or(u8::MAX);
    projector.stats.traversal.web_wrapper_nodes = counts.wrapper_nodes;
    projector.stats.traversal.cycles_skipped = counts.cycles_skipped;
    projector.stats.traversal.limits = counts.limits;
    projector.stats.reads.counts.observation_attempts = 1;
    projector.stats.reads.counts.child_reads = walk.node_count as u64;
    let subtree = projector.subtree(walk.root);
    (subtree, projector.stats)
}

#[cfg(test)]
#[path = "projection_tests.rs"]
mod tests;
