use super::*;
use agent_desktop_core::NotificationInfo;

fn info(index: usize, app: &str, title: &str) -> NotificationInfo {
    NotificationInfo {
        index,
        app_name: app.to_owned(),
        title: title.to_owned(),
        body: None,
        actions: Vec::new(),
    }
}

#[test]
fn survivors_count_duplicates() {
    let captured = [info(1, "M", "a"), info(2, "M", "a")];
    let current = [info(1, "M", "a")];
    assert_eq!(survivors(&captured, &current).len(), 1);
}

#[test]
fn a_closed_signal_alone_is_not_action_evidence() {
    assert_eq!(evidence(&[Signal::Closed(4)], true), Evidence::EntryRemoved);
}
