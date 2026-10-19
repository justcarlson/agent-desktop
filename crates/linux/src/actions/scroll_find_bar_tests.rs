use super::*;

fn node(parent: Option<usize>) -> Site {
    Site { parent, bar: None }
}

fn bar(parent: usize, way: Orientation) -> Site {
    Site {
        parent: Some(parent),
        bar: Some(way),
    }
}

#[test]
fn the_bar_beside_the_nearest_ancestor_of_the_target_wins() {
    let sites = [
        node(None),
        node(Some(0)),
        node(Some(1)),
        node(Some(2)),
        bar(0, Orientation::Vertical),
        bar(2, Orientation::Vertical),
    ];
    assert_eq!(choose_bar(&sites, Some(3), Axis::Vertical), Some(5));
}

#[test]
fn a_bar_beside_something_the_target_is_not_under_is_not_chosen() {
    let sites = [
        node(None),
        node(Some(0)),
        node(Some(0)),
        node(Some(1)),
        bar(2, Orientation::Vertical),
        bar(2, Orientation::Vertical),
    ];
    assert_eq!(choose_bar(&sites, Some(3), Axis::Vertical), None);
}

#[test]
fn the_axis_picks_between_two_bars_of_one_pane() {
    let sites = [
        node(None),
        node(Some(0)),
        bar(0, Orientation::Horizontal),
        bar(0, Orientation::Vertical),
    ];
    assert_eq!(choose_bar(&sites, Some(1), Axis::Vertical), Some(3));
    assert_eq!(choose_bar(&sites, Some(1), Axis::Horizontal), Some(2));
}

#[test]
fn a_target_the_crawl_never_met_uses_the_only_bar_that_runs_the_right_way() {
    let one = [node(None), bar(0, Orientation::Vertical)];
    assert_eq!(choose_bar(&one, None, Axis::Vertical), Some(1));
    let two = [
        node(None),
        bar(0, Orientation::Vertical),
        bar(0, Orientation::Unknown),
    ];
    assert_eq!(choose_bar(&two, None, Axis::Vertical), None);
    assert_eq!(choose_bar(&one, None, Axis::Horizontal), None);
}

#[test]
fn no_bar_at_all_is_none() {
    assert_eq!(
        choose_bar(&[node(None), node(Some(0))], Some(1), Axis::Vertical),
        None
    );
}

#[test]
fn a_bar_that_names_its_direction_beats_one_that_does_not_beside_the_same_pane() {
    let sites = [
        node(None),
        node(Some(0)),
        bar(0, Orientation::Unknown),
        bar(0, Orientation::Vertical),
    ];
    assert_eq!(choose_bar(&sites, Some(1), Axis::Vertical), Some(3));
}
