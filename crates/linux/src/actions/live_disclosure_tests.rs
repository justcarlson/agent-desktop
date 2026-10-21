use super::disclosure::set_expanded;
use super::live_click_tests::{act, find, open, state};
use crate::tree::state_bits::{EXPANDABLE, EXPANDED};

#[test]
#[ignore = "live: needs the AT-SPI session, Files (nautilus) with a folder tree and the write engine"]
fn a_files_tree_row_expands_and_collapses_in_both_directions() {
    let session = open("nautilus", &["--new-window", "/usr"]).expect("files opens");
    let Ok(row) = find(&session, |view| {
        view.state(EXPANDABLE) == Some(true) && view.state(EXPANDED) == Some(false)
    }) else {
        eprintln!("no collapsed expandable tree row in this Files layout; absence stated");
        return;
    };
    let opened = act(&session, &row, |target| set_expanded(target, true)).expect("expands");
    assert_eq!(opened.steps[0].verified(), Some(true));
    assert_eq!(state(&session, &row, EXPANDED), Some(true));
    let closed = act(&session, &row, |target| set_expanded(target, false)).expect("collapses");
    assert_eq!(closed.steps[0].verified(), Some(true));
    assert_eq!(state(&session, &row, EXPANDED), Some(false));
}
