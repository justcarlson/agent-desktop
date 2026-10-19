#[expect(dead_code, reason = "stub the owning lane replaces")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ItemProps {
    pub(crate) title: String,
    pub(crate) id: String,
    pub(crate) passive: bool,
    pub(crate) menu: Option<String>,
}

#[cfg(test)]
#[path = "item_props_tests.rs"]
mod tests;
