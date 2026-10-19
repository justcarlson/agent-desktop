use super::*;

fn labels(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_owned()).collect()
}

#[test]
fn entries_win_whatever_the_labels_say() {
    assert_eq!(
        classify(&labels(&["No Notifications"]), 2, 0),
        ContainerState::Entries
    );
}

#[test]
fn the_landmark_without_entries_is_empty() {
    assert_eq!(
        classify(&labels(&["Do Not Disturb", "No Notifications"]), 0, 0),
        ContainerState::Empty
    );
}

#[test]
fn neither_entries_nor_the_landmark_is_unrecognised() {
    assert_eq!(
        classify(&labels(&["Do Not Disturb"]), 0, 0),
        ContainerState::Unrecognised
    );
    assert_eq!(classify(&[], 0, 0), ContainerState::Unrecognised);
}

#[test]
fn a_footer_button_without_entries_is_an_empty_list_in_any_language() {
    assert_eq!(
        classify(&labels(&["Nicht stören"]), 0, 1),
        ContainerState::Empty
    );
}
