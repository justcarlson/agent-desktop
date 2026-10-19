#![expect(dead_code, reason = "stub the owning lane replaces")]

pub(crate) fn actions(_methods: &[String]) -> Vec<String> {
    Vec::new()
}

#[cfg(test)]
#[path = "capability_tests.rs"]
mod tests;
