use super::menu_popup::{MenuFamily, Node};

pub(super) struct Found<'a> {
    pub(super) root: &'a Node,
    pub(super) family: MenuFamily,
    pub(super) items: usize,
}
