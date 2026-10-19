#![expect(
    dead_code,
    reason = "the module is laid out ahead of the code that calls it"
)]

use super::release_state::ReleaseState;

pub(crate) struct ReleaseGuard {
    state: ReleaseState,
}

#[cfg(test)]
#[path = "release_guard_tests.rs"]
mod tests;
