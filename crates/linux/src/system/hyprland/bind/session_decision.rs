use std::time::Instant;

use super::bind_refusal::BindRefusal;
use super::bind_request::BindRequest;
use super::hyprland_binding::HyprlandBinding;

/// One binding decision, the request it answered and when it was made.
#[derive(Debug, Clone)]
pub(crate) struct SessionDecision {
    pub(crate) request: BindRequest,
    pub(crate) outcome: Result<Option<HyprlandBinding>, BindRefusal>,
    pub(crate) decided_at: Instant,
}
