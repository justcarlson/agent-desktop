use agent_desktop_core::{AdapterError, ErrorCode};
use serde_json::json;

/// Holds the active window steady across a pointer sequence. The first
/// check takes the baseline. The second check, which follows the first
/// press, takes it again once, because that press may focus the window
/// under the point; with an `expected` window that new baseline must be
/// it. Every later check refuses a change.
pub(crate) struct FocusGate<R> {
    read: R,
    expected: Option<u64>,
    baseline: Option<Option<u64>>,
    checks: u32,
}

impl<R: FnMut() -> Result<Option<u64>, AdapterError>> FocusGate<R> {
    pub(crate) fn new(read: R, expected: Option<u64>) -> Self {
        Self {
            read,
            expected,
            baseline: None,
            checks: 0,
        }
    }

    pub(crate) fn check(&mut self) -> Result<(), AdapterError> {
        let now = (self.read)()?;
        self.checks = self.checks.saturating_add(1);
        match (self.baseline, self.checks) {
            (None, _) => {
                self.baseline = Some(now);
                Ok(())
            }
            (Some(_), 2) => match self.expected {
                Some(expected) if now != Some(expected) => Err(changed(Some(expected), now)),
                _ => {
                    self.baseline = Some(now);
                    Ok(())
                }
            },
            (Some(baseline), _) if baseline != now => Err(changed(baseline, now)),
            (Some(_), _) => Ok(()),
        }
    }
}

fn changed(from: Option<u64>, to: Option<u64>) -> AdapterError {
    let address = |window: Option<u64>| window.map(|address| format!("0x{address:x}"));
    AdapterError::new(
        ErrorCode::ActionFailed,
        "The active window changed during pointer input",
    )
    .with_details(json!({
        "kind": "focus_gate",
        "cause": "focus_changed",
        "from": address(from),
        "to": address(to),
    }))
    .with_suggestion("Run 'snapshot' to read the real state before retrying")
}

#[cfg(test)]
#[path = "focus_gate_tests.rs"]
mod tests;
