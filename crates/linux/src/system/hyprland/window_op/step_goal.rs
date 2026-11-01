use serde_json::{Value, json};

use super::client_state::ClientState;
use crate::system::window::window_op::window_op_verdict::POSITION_TOLERANCE;

/// One property a dispatch must leave on the window, read back from
/// `j/clients`. Places and sizes hold within `POSITION_TOLERANCE` pixels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StepGoal {
    At(i64, i64),
    Size(i64, i64),
    Workspace(String),
    Fullscreen(i64),
}

impl StepGoal {
    pub(crate) fn holds(&self, state: &ClientState) -> bool {
        match self {
            Self::At(x, y) => near((*x, *y), state.at),
            Self::Size(width, height) => near((*width, *height), state.size),
            Self::Workspace(name) => state.workspace == *name,
            Self::Fullscreen(mode) => state.fullscreen == *mode,
        }
    }

    pub(crate) fn to_json(&self) -> Value {
        match self {
            Self::At(x, y) => json!({ "x": x, "y": y }),
            Self::Size(width, height) => json!({ "width": width, "height": height }),
            Self::Workspace(name) => json!({ "workspace": name }),
            Self::Fullscreen(mode) => json!({ "fullscreen": mode }),
        }
    }
}

fn near(wanted: (i64, i64), seen: (i64, i64)) -> bool {
    let tolerance = i64::from(POSITION_TOLERANCE);
    (wanted.0 - seen.0).abs() <= tolerance && (wanted.1 - seen.1).abs() <= tolerance
}
