#![expect(
    dead_code,
    reason = "the module is laid out ahead of the code that calls it"
)]

#[derive(Debug, Default)]
pub(crate) struct ReleaseState {
    armed: bool,
    delivered: u32,
}

#[cfg(test)]
#[path = "release_state_tests.rs"]
mod tests;
