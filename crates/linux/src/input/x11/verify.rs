#![expect(
    dead_code,
    reason = "the module is laid out ahead of the code that calls it"
)]

use super::sink::PointerRead;

pub(crate) fn pointer_is_clear(_read: &PointerRead) -> bool {
    true
}

#[cfg(test)]
#[path = "verify_tests.rs"]
mod tests;
