#![expect(dead_code, reason = "stub the owning lane replaces")]

use super::menu_layout::LayoutEntry;

pub(crate) fn child_count(_entries: &[LayoutEntry]) -> usize {
    0
}

#[cfg(test)]
#[path = "menu_nodes_tests.rs"]
mod tests;
