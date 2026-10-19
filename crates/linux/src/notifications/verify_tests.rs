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
fn identity_ignores_index_and_actions() {
    let mut other = info(9, "Mail", "a");
    other.actions.push("Go".to_owned());
    assert!(same_identity(&info(1, "Mail", "a"), &other));
    assert!(!same_identity(&info(1, "Mail", "a"), &info(1, "Mail", "b")));
}

#[test]
fn lost_matches_duplicates_one_for_one() {
    let before = [info(1, "M", "a"), info(2, "M", "a"), info(3, "M", "b")];
    let after = [info(1, "M", "a"), info(2, "M", "b")];
    let gone = lost(&before, &after);
    assert_eq!(gone.len(), 1);
    assert_eq!(gone[0].title, "a");
}

#[test]
fn removal_is_settled_only_when_exactly_the_target_left() {
    let target = info(2, "M", "b");
    let before = [info(1, "M", "a"), info(2, "M", "b"), info(3, "M", "c")];
    assert_eq!(judge_removal(&before, &before, &target), Removal::Pending);
    assert_eq!(
        judge_removal(&before, &[before[0].clone(), before[2].clone()], &target),
        Removal::Settled
    );
    assert_eq!(
        judge_removal(&before, &[before[0].clone()], &target),
        Removal::Bystander
    );
    assert_eq!(
        judge_removal(&before, &[before[1].clone(), before[2].clone()], &target),
        Removal::Bystander
    );
}

#[test]
fn a_second_identical_entry_leaving_is_a_bystander() {
    let target = info(2, "M", "a");
    let before = [info(1, "M", "a"), info(2, "M", "a")];
    assert_eq!(
        judge_removal(&before, &[before[0].clone()], &target),
        Removal::Settled
    );
    assert_eq!(judge_removal(&before, &[], &target), Removal::Bystander);
}

#[test]
fn new_arrivals_are_not_survivors() {
    let captured = [info(1, "M", "a")];
    let current = [info(1, "M", "new")];
    assert!(survivors(&captured, &current).is_empty());
}

#[test]
fn survivors_count_duplicates() {
    let captured = [info(1, "M", "a"), info(2, "M", "a")];
    let current = [info(1, "M", "a")];
    assert_eq!(survivors(&captured, &current).len(), 1);
}

#[test]
fn evidence_names_the_signal_only_when_single_and_the_entry_is_gone() {
    let one = [Signal::ActionInvoked(4), Signal::Closed(4)];
    assert_eq!(evidence(&one, true), Evidence::ActionInvoked);
    assert_eq!(evidence(&[], true), Evidence::EntryRemoved);
    assert_eq!(evidence(&[], false), Evidence::SettleElapsed);
    assert_eq!(evidence(&one, false), Evidence::SettleElapsed);
}

#[test]
fn a_second_action_signal_ends_the_signal_claim() {
    let two = [Signal::ActionInvoked(4), Signal::ActionInvoked(9)];
    assert_eq!(evidence(&two, true), Evidence::EntryRemoved);
}

#[test]
fn a_closed_signal_alone_is_not_action_evidence() {
    assert_eq!(evidence(&[Signal::Closed(4)], true), Evidence::EntryRemoved);
}
