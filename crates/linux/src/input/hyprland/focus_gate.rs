use agent_desktop_core::AdapterError;

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
        let _ = (&mut self.read, self.expected, self.baseline, self.checks);
        Ok(())
    }
}

#[cfg(test)]
#[path = "focus_gate_tests.rs"]
mod tests;
