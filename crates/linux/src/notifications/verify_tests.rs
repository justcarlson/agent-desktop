use super::*;
use crate::notifications::fake::element;
use agent_desktop_core::NotificationInfo;

fn seen(index: usize, app: &str, title: &str, object: &str) -> Seen {
    Seen {
        info: NotificationInfo {
            index,
            app_name: app.to_owned(),
            title: title.to_owned(),
            body: None,
            actions: Vec::new(),
        },
        node: element(object),
    }
}

#[test]
fn bystanders_are_found_by_object_and_never_by_words() {
    let target = seen(1, "M", "a", "/one");
    let before = [
        target.clone(),
        seen(2, "M", "a", "/two"),
        seen(3, "C", "b", "/three"),
    ];
    let after = [seen(1, "M", "a", "/two")];
    let gone = bystanders(&before, &after, &target);
    assert_eq!(gone.len(), 1);
    assert_eq!(gone[0].node.path(), "/three");
}

#[test]
fn departed_and_remaining_split_the_captured_set_by_object() {
    let captured = [seen(1, "M", "a", "/one"), seen(2, "M", "a", "/two")];
    let current = [seen(1, "M", "a", "/two")];
    assert_eq!(departed(&captured, &current).len(), 1);
    assert_eq!(remaining(&captured, &current).len(), 1);
    assert_eq!(remaining(&captured, &current)[0].node.path(), "/two");
}

#[test]
fn an_action_needs_the_entry_to_have_left_to_count() {
    assert_eq!(evidence(&[Signal::ActionInvoked(7)], false), None);
    assert_eq!(evidence(&[], false), None);
}

#[test]
fn a_closed_signal_alone_is_not_action_evidence() {
    assert_eq!(
        evidence(&[Signal::Closed(4)], true),
        Some(Evidence::EntryRemoved)
    );
}

#[test]
fn one_action_signal_with_the_entry_gone_names_the_action() {
    assert_eq!(
        evidence(&[Signal::ActionInvoked(7)], true),
        Some(Evidence::ActionInvoked)
    );
    assert_eq!(
        evidence(&[Signal::ActionInvoked(7), Signal::ActionInvoked(9)], true),
        Some(Evidence::EntryRemoved)
    );
}
