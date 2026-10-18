use super::value_read::ValueRead;
use super::{AtspiElement, LabelOutcome, RawNode};

/// Everything read for one node of an observation: the raw node, the text of
/// its label when its name is blank, and its value when its role carries one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ObservedRead {
    pub(crate) node: RawNode,
    pub(crate) label: LabelOutcome,
    pub(crate) value: ValueRead,
}

impl ObservedRead {
    /// A node whose every read failed, kept in place so the walk can go on.
    pub(crate) fn unreadable(element: AtspiElement, detail: &str) -> Self {
        Self {
            node: RawNode::unreadable(element, detail),
            label: LabelOutcome::Unlabelled,
            value: ValueRead::absent(),
        }
    }

    /// A node read without a label or a value.
    #[cfg(test)]
    pub(crate) fn plain(node: RawNode) -> Self {
        Self {
            node,
            label: LabelOutcome::Unlabelled,
            value: ValueRead::absent(),
        }
    }
}
