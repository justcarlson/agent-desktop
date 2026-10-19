#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LayoutEntry {
    pub(crate) id: i32,
    pub(crate) label: String,
    pub(crate) enabled: bool,
    pub(crate) separator: bool,
    pub(crate) children: Vec<LayoutEntry>,
}

#[cfg(test)]
#[path = "menu_layout_tests.rs"]
mod tests;
