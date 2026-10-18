use agent_desktop_core::{IdentifierEvidence, LocatorField, NodeDescriptor};

use super::RawNode;
use super::descriptor::descriptors;
use super::identity::identifier_evidence;
use super::label_outcome::LabelOutcome;
use super::name_evidence::name_fields;
use super::placeholder::read_placeholder;
use super::roles::resolve_role;
use super::states::resolve_states;

/// What a node means, in core's own types.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ResolvedVocabulary {
    pub(crate) role: LocatorField<String>,
    pub(crate) states: LocatorField<Vec<String>>,
    pub(crate) name: LocatorField<String>,
    pub(crate) description: LocatorField<String>,
    pub(crate) identifiers: IdentifierEvidence,
    pub(crate) descriptor: NodeDescriptor,
}

/// Resolves the vocabulary of one raw node. `in_web_document` is true when
/// the node sits below a web document, which the walk knows from its path.
#[allow(dead_code)]
pub(crate) fn resolve_vocabulary(
    node: &RawNode,
    label: &LabelOutcome,
    in_web_document: bool,
) -> ResolvedVocabulary {
    let traits = &node.traits;
    let placeholder = read_placeholder(&node.properties, &traits.role, &node.attributes);
    let (name, description) = name_fields(&node.properties, &traits.role, &placeholder, label);
    ResolvedVocabulary {
        role: resolve_role(traits, &node.attributes),
        states: resolve_states(&traits.role, &traits.states),
        name,
        description,
        identifiers: identifier_evidence(
            &node.properties.accessible_id,
            &node.attributes,
            in_web_document,
        ),
        descriptor: descriptors(
            &traits.role,
            &traits.role_name,
            &placeholder,
            &node.attributes,
            in_web_document,
        ),
    }
}
