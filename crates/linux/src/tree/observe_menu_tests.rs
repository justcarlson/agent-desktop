use super::rooted;
use crate::tree::AtspiElement;

#[test]
fn the_popup_root_is_its_own_frame_and_not_the_window() {
    let root = rooted(&AtspiElement::new(":1.2", "/popup", 7));
    assert_eq!(root.frame().map(AtspiElement::path), Some("/popup"));
}

mod pruning {
    use super::super::{BULK_CHILDREN, is_cut_short_beyond_pruning, is_pruned};
    use crate::tree::role_numbers::{LIST, PANEL, TABLE};
    use crate::tree::test_support::read_of;
    use crate::tree::{ObservedRead, WalkedNode};

    fn node(role: u32, children: Vec<WalkedNode<ObservedRead>>) -> WalkedNode<ObservedRead> {
        WalkedNode {
            payload: read_of(role, "n", &[]),
            children,
            truncated: false,
            child_count: None,
            source_index: 0,
            in_web_document: false,
        }
    }

    fn cut_list(role: u32, total: u32) -> WalkedNode<ObservedRead> {
        let rows = (0..BULK_CHILDREN)
            .map(|_| node(PANEL, Vec::new()))
            .collect();
        let mut list = node(role, rows);
        list.truncated = true;
        list.child_count = Some(total);
        list
    }

    #[test]
    fn a_list_or_table_cut_at_the_child_limit_is_pruned_and_not_cut_short() {
        for role in [LIST, TABLE] {
            let list = cut_list(role, 3000);
            assert!(is_pruned(&list));
            assert!(!is_cut_short_beyond_pruning(&node(PANEL, vec![list])));
        }
    }

    #[test]
    fn a_container_that_is_not_a_list_cut_at_the_limit_is_cut_short() {
        let panel = cut_list(PANEL, 3000);
        assert!(!is_pruned(&panel));
        assert!(is_cut_short_beyond_pruning(&panel));
    }

    #[test]
    fn a_list_whose_read_stopped_short_of_the_limit_is_cut_short() {
        let mut list = cut_list(LIST, 3000);
        list.children.truncate(10);
        assert!(!is_pruned(&list));
        assert!(is_cut_short_beyond_pruning(&list));
    }

    #[test]
    fn a_truncation_below_a_pruned_list_still_counts() {
        let mut list = cut_list(LIST, 3000);
        list.children[3].truncated = true;
        assert!(is_cut_short_beyond_pruning(&list));
    }
}
