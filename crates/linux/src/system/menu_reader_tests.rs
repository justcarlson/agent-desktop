use super::may_hide_a_menu;
use crate::system::menu_fixture::*;

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
