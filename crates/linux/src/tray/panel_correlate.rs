#![expect(dead_code, reason = "stub the owning lane replaces")]

pub(crate) fn pair(_items: &[String], _panel_titles: &[String]) -> Vec<Option<usize>> {
    Vec::new()
}

#[cfg(test)]
#[path = "panel_correlate_tests.rs"]
mod tests;
