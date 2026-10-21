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
