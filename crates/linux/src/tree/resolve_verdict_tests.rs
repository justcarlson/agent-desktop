use super::*;
use crate::tree::test_support::rect;

fn twins() -> Vec<Option<Rect>> {
    vec![
        Some(rect(391.0, 196.0, 34.0, 34.0)),
        Some(rect(430.0, 196.0, 23.0, 34.0)),
    ]
}

#[test]
fn the_stored_size_picks_the_twin_it_belongs_to() {
    let first = rect(10.0, 10.0, 34.0, 34.0);
    let second = rect(10.0, 10.0, 23.0, 34.0);
    assert_eq!(
        select_by_stored_size(&twins(), &[], Some(&first), BoundsSpace::Screen),
        Some(0)
    );
    assert_eq!(
        select_by_stored_size(&twins(), &[], Some(&second), BoundsSpace::Screen),
        Some(1)
    );
}

#[test]
fn a_size_that_matches_neither_or_no_size_at_all_separates_nothing() {
    let neither = rect(10.0, 10.0, 40.0, 34.0);
    assert_eq!(
        select_by_stored_size(&twins(), &[], Some(&neither), BoundsSpace::Screen),
        None
    );
    assert_eq!(
        select_by_stored_size(&twins(), &[], None, BoundsSpace::Screen),
        None
    );
    let flat = rect(10.0, 10.0, 34.0, 0.0);
    assert_eq!(
        select_by_stored_size(&twins(), &[], Some(&flat), BoundsSpace::Screen),
        None
    );
}

#[test]
fn two_candidates_of_the_stored_size_stay_ambiguous() {
    let same = vec![
        Some(rect(0.0, 0.0, 34.0, 34.0)),
        Some(rect(50.0, 0.0, 34.0, 34.0)),
    ];
    let stored = rect(0.0, 0.0, 34.0, 34.0);
    assert_eq!(
        select_by_stored_size(&same, &[], Some(&stored), BoundsSpace::Screen),
        None
    );
}

#[test]
fn position_is_never_read_so_a_moved_window_selects_the_same_twin() {
    let stored = rect(391.0, 196.0, 34.0, 34.0);
    let moved: Vec<Option<Rect>> = twins()
        .into_iter()
        .map(|candidate| {
            candidate.map(|rect| Rect {
                x: rect.x - 89.0,
                y: rect.y + 64.0,
                ..rect
            })
        })
        .collect();
    let at_origin: Vec<Option<Rect>> = twins()
        .into_iter()
        .map(|candidate| {
            candidate.map(|rect| Rect {
                x: 0.0,
                y: 0.0,
                ..rect
            })
        })
        .collect();
    assert_eq!(
        select_by_stored_size(&moved, &[], Some(&stored), BoundsSpace::Screen),
        Some(0)
    );
    assert_eq!(
        select_by_stored_size(&at_origin, &[], Some(&stored), BoundsSpace::Screen),
        Some(0)
    );
}

#[test]
fn a_candidate_with_unreadable_extents_never_lets_its_twin_win_by_size() {
    let candidates = vec![None, Some(rect(0.0, 0.0, 23.0, 34.0))];
    let stored = rect(0.0, 0.0, 23.0, 34.0);
    assert_eq!(
        select_by_stored_size(&candidates, &[], Some(&stored), BoundsSpace::Screen),
        None
    );
    assert_eq!(
        classify(&candidates, &[], Some(&stored), BoundsSpace::Screen, false),
        Verdict::Incomplete
    );
    let readable = vec![
        Some(rect(0.0, 0.0, 34.0, 34.0)),
        Some(rect(0.0, 0.0, 23.0, 34.0)),
    ];
    assert_eq!(
        classify(&readable, &[], Some(&stored), BoundsSpace::Screen, false),
        Verdict::Resolved(1)
    );
}

#[test]
fn two_matches_with_no_usable_stored_size_are_ambiguous_even_with_an_unread_region() {
    assert_eq!(
        classify(&twins(), &[], None, BoundsSpace::Screen, true),
        Verdict::Ambiguous
    );
    assert_eq!(
        classify(&twins(), &[], None, BoundsSpace::Screen, false),
        Verdict::Ambiguous
    );
}

#[test]
fn a_lone_match_beside_an_unread_region_is_incomplete_and_resolves_once_everything_was_read() {
    let one = vec![Some(rect(0.0, 0.0, 34.0, 34.0))];
    assert_eq!(
        classify(&one, &[], None, BoundsSpace::Screen, true),
        Verdict::Incomplete
    );
    assert_eq!(
        classify(&one, &[], None, BoundsSpace::Screen, false),
        Verdict::Resolved(0)
    );
}

#[test]
fn no_match_is_stale_only_when_every_region_was_read() {
    assert_eq!(
        classify(&[], &[], None, BoundsSpace::Screen, false),
        Verdict::Stale
    );
    assert_eq!(
        classify(&[], &[], None, BoundsSpace::Screen, true),
        Verdict::Incomplete
    );
}

#[test]
fn a_stored_size_resolves_a_pair_only_when_nothing_is_left_unread() {
    let stored = rect(0.0, 0.0, 23.0, 34.0);
    assert_eq!(
        classify(&twins(), &[], Some(&stored), BoundsSpace::Screen, false),
        Verdict::Resolved(1)
    );
    assert_eq!(
        classify(&twins(), &[], Some(&stored), BoundsSpace::Screen, true),
        Verdict::Incomplete
    );
    let neither = rect(0.0, 0.0, 99.0, 34.0);
    assert_eq!(
        classify(&twins(), &[], Some(&neither), BoundsSpace::Screen, false),
        Verdict::Ambiguous
    );
}
