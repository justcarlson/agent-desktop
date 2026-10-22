use super::may_hide_a_menu;
use crate::system::menu::menu_fixture::*;

#[test]
fn a_whole_read_hides_no_menu() {
    assert!(!may_hide_a_menu(&chromium_frame()));
}

#[test]
fn a_cut_short_read_of_menu_content_may_hide_one() {
    let mut cut = chromium_frame();
    cut.truncated = true;
    assert!(may_hide_a_menu(&cut));
}

#[test]
fn a_cut_short_read_that_shows_other_content_hides_no_menu() {
    let mut cut = node(
        FRAME,
        "",
        &[SHOWING],
        vec![node(LABEL, "hello", &[SHOWING], Vec::new())],
    );
    cut.truncated = true;
    assert!(!may_hide_a_menu(&cut));
}

#[test]
fn a_structural_inventory_reads_every_walk_structurally_and_keeps_the_pruning() {
    use super::{BULK_CHILDREN, ScanKind, scan_kind};
    assert_eq!(
        scan_kind(true, false),
        ScanKind::Structural {
            bulk_children: None
        }
    );
    assert_eq!(
        scan_kind(true, true),
        ScanKind::Structural {
            bulk_children: Some(BULK_CHILDREN)
        }
    );
    assert_eq!(scan_kind(false, true), ScanKind::Pruned);
    assert_eq!(scan_kind(false, false), ScanKind::Ordinary);
}
