use std::sync::{Mutex, MutexGuard};

static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

/// Held by a live test for its whole run. The tests of one process share one
/// private display, and a test that paints or grabs the whole screen sees the
/// windows of every test running beside it.
pub(crate) fn exclusive() -> MutexGuard<'static, ()> {
    ONE_AT_A_TIME
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub(crate) struct Private {
    _guard: std::sync::MutexGuard<'static, ()>,
    name: String,
}

impl std::ops::Deref for Private {
    type Target = String;

    fn deref(&self) -> &String {
        &self.name
    }
}

pub(crate) fn private_display() -> Private {
    let guard = exclusive();
    let display = std::env::var("DISPLAY").expect("DISPLAY");
    let private = std::env::var("AGENT_DESKTOP_PRIVATE_SESSION").expect("a private session");
    assert!(!private.is_empty() && !matches!(display.as_str(), ":0" | ":99" | ""));
    Private {
        _guard: guard,
        name: display,
    }
}
