use std::hash::{Hash, Hasher};

use agent_desktop_core::{LocatorEvidence, ObservedSubtree, RefEntry};

use super::decorate::Decorate;
use super::{ObservedRead, WalkedNode};

const PREFIX: &str = "linux-menu:v1:";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MenuContext {
    root: String,
    shape: u64,
    complete: bool,
}

impl MenuContext {
    pub(crate) fn from_walk(root: &WalkedNode<ObservedRead>) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        hash_node(root, &mut hasher);
        Self {
            root: root.payload.node.element.path().to_owned(),
            shape: hasher.finish(),
            complete: tree_complete(root),
        }
    }

    pub(crate) fn matches(&self, other: &Self) -> bool {
        self.complete && other.complete && self == other
    }

    pub(crate) fn entry_for(&self, entry: &RefEntry) -> Option<RefEntry> {
        let token = entry.identity.retained_object.as_deref()?;
        let (expected, object) = Self::decode(token)?;
        if !expected.matches(self) || object.is_empty() {
            return None;
        }
        let mut scoped = entry.clone();
        scoped.identity.retained_object = Some(object.to_owned());
        Some(scoped)
    }

    pub(crate) fn encode(&self, object: &str) -> String {
        format!(
            "{PREFIX}{}:{}:{}:{}:{}",
            self.shape,
            u8::from(self.complete),
            self.root.len(),
            self.root,
            object
        )
    }

    #[cfg(test)]
    pub(crate) fn test(root: &str, shape: u64) -> Self {
        Self {
            root: root.to_owned(),
            shape,
            complete: true,
        }
    }

    pub(crate) fn decode(token: &str) -> Option<(Self, &str)> {
        let rest = token.strip_prefix(PREFIX)?;
        let (shape, rest) = rest.split_once(':')?;
        let (complete, rest) = rest.split_once(':')?;
        let (root_len, rest) = rest.split_once(':')?;
        let shape = shape.parse().ok()?;
        let complete = match complete {
            "0" => false,
            "1" => true,
            _ => return None,
        };
        let root_len: usize = root_len.parse().ok()?;
        if rest.len() < root_len || !rest.is_char_boundary(root_len) {
            return None;
        }
        let root = rest[..root_len].to_owned();
        let rest = rest[root_len..].strip_prefix(':')?;
        let (root, object) = (root, rest);
        Some((
            Self {
                root,
                shape,
                complete,
            },
            object,
        ))
    }
}

fn tree_complete(node: &WalkedNode<ObservedRead>) -> bool {
    !node.truncated
        && node.payload.label != super::LabelOutcome::Failed
        && node.children.iter().all(tree_complete)
}

fn hash_node(node: &WalkedNode<ObservedRead>, hasher: &mut impl Hasher) {
    node.payload.node.element.path().hash(hasher);
    node.payload.node.traits.role.present().hash(hasher);
    node.payload.node.properties.name.present().hash(hasher);
    node.payload.label.hash(hasher);
    node.truncated.hash(hasher);
    node.child_count.hash(hasher);
    node.children.len().hash(hasher);
    for child in &node.children {
        hash_node(child, hasher);
    }
}

pub(crate) struct MenuIdentity {
    context: MenuContext,
}

impl MenuIdentity {
    pub(crate) fn new(context: MenuContext) -> Self {
        Self { context }
    }
}

impl Decorate for MenuIdentity {
    fn decorate(
        &mut self,
        read: &ObservedRead,
        evidence: &mut LocatorEvidence,
        _first_child_index: usize,
    ) -> Vec<ObservedSubtree> {
        evidence.identifiers = evidence
            .identifiers
            .clone()
            .with_retained_object(self.context.encode(read.node.element.path()));
        Vec::new()
    }
}

#[cfg(test)]
#[path = "menu_context_tests.rs"]
mod tests;
