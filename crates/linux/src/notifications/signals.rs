#![expect(dead_code, reason = "stub the owning lane replaces")]

use std::time::Duration;

pub(crate) const SETTLE_BUDGET: Duration = Duration::from_millis(0);

#[cfg(test)]
#[path = "signals_tests.rs"]
mod tests;
