use super::{decide_focus, focus_is_indeterminate};
use crate::system::x11::X11Evidence;

#[test]
fn one_active_candidate_without_a_window_manager_is_focused() {
    assert_eq!(decide_focus(&[3], X11Evidence::Unavailable), Some(3));
}

#[test]
fn two_active_candidates_without_a_window_manager_focus_none() {
    assert_eq!(decide_focus(&[1, 3], X11Evidence::Unavailable), None);
}

#[test]
fn two_candidates_with_the_window_manager_naming_one_focus_that_one() {
    assert_eq!(decide_focus(&[1, 3], X11Evidence::Frame(3)), Some(3));
}

#[test]
fn a_window_manager_naming_a_different_window_focuses_none() {
    assert_eq!(decide_focus(&[1], X11Evidence::Frame(2)), None);
}

#[test]
fn an_unmapped_or_absent_active_window_focuses_none() {
    for evidence in [X11Evidence::Unmapped, X11Evidence::NoActive] {
        assert_eq!(decide_focus(&[1], evidence), None, "{evidence:?}");
    }
}

#[test]
fn no_candidate_focuses_none_whatever_the_window_manager_says() {
    for evidence in [
        X11Evidence::Unavailable,
        X11Evidence::Frame(0),
        X11Evidence::Ambiguous,
        X11Evidence::Unmapped,
        X11Evidence::NoActive,
    ] {
        assert_eq!(decide_focus(&[], evidence), None, "{evidence:?}");
    }
}

#[test]
fn an_active_candidate_that_no_source_confirms_is_indeterminate() {
    let cases = [
        (vec![1, 3], X11Evidence::Unavailable),
        (vec![1], X11Evidence::NoActive),
        (vec![1], X11Evidence::Unmapped),
        (vec![1], X11Evidence::Frame(2)),
    ];
    for (candidates, evidence) in cases {
        let decided = decide_focus(&candidates, evidence);
        assert!(
            focus_is_indeterminate(&candidates, decided),
            "{candidates:?} {evidence:?}"
        );
    }
}

#[test]
fn a_decided_or_candidate_free_focus_is_not_indeterminate() {
    let cases = [
        (vec![1, 3], X11Evidence::Frame(3)),
        (vec![3], X11Evidence::Unavailable),
        (vec![3], X11Evidence::Ambiguous),
        (vec![], X11Evidence::Unavailable),
        (vec![], X11Evidence::Ambiguous),
        (vec![], X11Evidence::NoActive),
    ];
    for (candidates, evidence) in cases {
        let decided = decide_focus(&candidates, evidence);
        assert!(
            !focus_is_indeterminate(&candidates, decided),
            "{candidates:?} {evidence:?}"
        );
    }
}

#[test]
fn one_active_candidate_among_same_titled_windows_is_focused() {
    assert_eq!(decide_focus(&[1], X11Evidence::Ambiguous), Some(1));
    assert_eq!(decide_focus(&[1, 3], X11Evidence::Ambiguous), None);
}
