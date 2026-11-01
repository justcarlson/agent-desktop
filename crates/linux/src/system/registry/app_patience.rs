use std::time::Duration;

use crate::system::registry::read_patience::ReadPatience;
use crate::system::registry::registry_frames::{CHILDREN_SLICE, FRAME_SLICE};

/// The grace a running application gets past a slice. A GTK window that
/// draws slowly answers its accessibility calls only between draws.
pub(crate) const BUSY_GRACE: Duration = Duration::from_millis(700);

/// The waits for one application's child list and for each of its frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AppPatience {
    pub(crate) children: ReadPatience,
    pub(crate) frames: ReadPatience,
}

impl AppPatience {
    pub(crate) fn for_process(pid: u32) -> Self {
        Self {
            children: ReadPatience::for_process(CHILDREN_SLICE, BUSY_GRACE, pid),
            frames: ReadPatience::for_process(FRAME_SLICE, BUSY_GRACE, pid),
        }
    }
}
