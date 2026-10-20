use serde::{Deserialize, Serialize};

use crate::{Point, ProcessIdentity};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OperationTarget {
    Window {
        id: String,
        title: String,
        process: ProcessIdentity,
    },
    App {
        name: String,
        process: ProcessIdentity,
    },
    Pointer {
        point: Point,
    },
    Drag {
        from: Point,
        to: Point,
    },
    Clipboard,
}
