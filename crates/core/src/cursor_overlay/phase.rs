use serde::{Deserialize, Serialize};

/// Which half of an action a cursor instruction represents.
///
/// `Travel` is sent before an action and acknowledged once the cursor lands.
/// `Drag` arms pointer tracking and is acknowledged once the renderer is ready.
/// `Effect` is sent after dispatch; the renderer acknowledges it, and the
/// action result does not depend on that acknowledgement.
#[derive(Debug, Clone, Copy, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CursorPhase {
    #[default]
    Travel,
    Drag,
    Effect,
}
