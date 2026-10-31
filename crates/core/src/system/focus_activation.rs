use serde::{Deserialize, Serialize};

use super::focus_proof::FocusProof;
use super::focus_side_effect::FocusSideEffect;

/// How `focus-window` reached a focused window: whether it sent an
/// activation, which observers proved the focus, and what else changed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FocusActivation {
    pub activated: bool,
    pub proof: FocusProof,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub side_effects: Vec<FocusSideEffect>,
}
