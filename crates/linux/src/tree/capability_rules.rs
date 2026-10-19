use super::RawNode;

pub(crate) fn scroll_tokens(_node: &RawNode) -> Vec<&'static str> {
    Vec::new()
}

#[cfg(test)]
#[path = "capability_rules_tests.rs"]
mod tests;
