use agent_desktop_core::{AdapterError, ErrorCode, LocatorEvidence};
use zbus::Connection;

use super::absence::Liveness;
use super::projection::project_node;
use super::walker_expand::read_observed_with_liveness;
use super::{AtspiElement, ObservedRead, WalkLimits};

/// One node read once: the walk's own read, the evidence it projects to, and
/// whether the node's role call answered in time.
#[derive(Debug, Clone)]
pub(crate) struct NodeEvidence {
    #[expect(
        dead_code,
        reason = "the live reads project state, value and actions from this same read"
    )]
    pub(crate) read: ObservedRead,
    pub(crate) evidence: LocatorEvidence,
    pub(crate) liveness: Liveness,
}

fn missing_ancestry() -> AdapterError {
    AdapterError::new(
        ErrorCode::InvalidArgs,
        "The element handle carries no window frame, so its web context is unknown",
    )
    .with_details(serde_json::json!({ "kind": "invalid_native_handle", "platform": "linux" }))
}

/// A handle that came from a walk or a descent names the window frame it sits
/// under. One without it is refused: reading it would guess whether a web
/// document encloses it, and the guess decides whether its DOM identifier is
/// read at all.
pub(crate) fn require_ancestry(element: &AtspiElement) -> Result<(), AdapterError> {
    element.frame().map(|_| ()).ok_or_else(missing_ancestry)
}

/// The evidence of a node read for `element`, in the web context the element
/// carries.
pub(crate) fn evidence_for(read: &ObservedRead, element: &AtspiElement) -> LocatorEvidence {
    project_node(read, element.in_web_document(), &WalkLimits::default())
}

/// Reads one node and projects it exactly as the snapshot walk does, so the
/// identity read later equals the identity a ref was taken with.
pub(crate) async fn read_evidence(
    connection: &Connection,
    element: &AtspiElement,
) -> Result<NodeEvidence, AdapterError> {
    require_ancestry(element)?;
    let (read, liveness) =
        read_observed_with_liveness(connection, element, WalkLimits::default().max_field_bytes)
            .await?;
    let evidence = evidence_for(&read, element);
    Ok(NodeEvidence {
        read,
        evidence,
        liveness,
    })
}

#[cfg(test)]
#[path = "node_evidence_tests.rs"]
mod tests;
