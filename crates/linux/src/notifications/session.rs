#![expect(dead_code, reason = "stub the owning lane replaces")]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Restore {
    Adopted,
    Closed,
    Unverified,
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
