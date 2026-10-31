use serde::{Deserialize, Serialize};

use crate::Point;

/// A change to the desktop that focusing a window caused beyond the focus
/// itself. Workspaces are named as the compositor names them; cursor
/// positions are in global compositor coordinates.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FocusSideEffect {
    WorkspaceSwitched {
        monitor: String,
        from: String,
        to: String,
    },
    SpecialWorkspaceRevealed {
        monitor: String,
        workspace: String,
    },
    SpecialWorkspaceClosed {
        monitor: String,
        workspace: String,
    },
    CursorMoved {
        from: Point,
        to: Point,
    },
}
