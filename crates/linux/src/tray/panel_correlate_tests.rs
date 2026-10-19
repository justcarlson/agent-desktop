use super::*;

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

#[test]
fn unique_titles_pair_to_their_item_whatever_the_order() {
    let items = strings(&["Dropbox", "Slack", "Steam"]);
    let panel = strings(&["Steam", "Dropbox", "Slack"]);
    assert_eq!(pair(&items, &panel), vec![Some(2), Some(0), Some(1)]);
}

#[test]
fn a_title_repeated_among_items_pairs_with_no_panel_node() {
    let items = strings(&["Twin", "Twin", "Solo"]);
    let panel = strings(&["Twin", "Solo"]);
    assert_eq!(pair(&items, &panel), vec![None, Some(2)]);
}

#[test]
fn a_title_repeated_in_the_panel_pairs_with_no_item() {
    let items = strings(&["Twin", "Solo"]);
    let panel = strings(&["Twin", "Twin", "Solo"]);
    assert_eq!(pair(&items, &panel), vec![None, None, Some(1)]);
}

#[test]
fn a_panel_only_title_has_no_pairing() {
    let items = strings(&["Solo"]);
    let panel = strings(&["AnyDesk", "Solo"]);
    assert_eq!(pair(&items, &panel), vec![None, Some(0)]);
}

#[test]
fn an_empty_title_never_pairs() {
    assert_eq!(pair(&strings(&[""]), &strings(&[""])), vec![None]);
}

#[test]
fn nothing_on_either_side_pairs_to_nothing() {
    assert!(pair(&[], &[]).is_empty());
    assert_eq!(pair(&[], &strings(&["A"])), vec![None]);
}
