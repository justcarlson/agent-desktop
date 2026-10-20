use super::*;

fn app(pid: u32, name: &str) -> Running {
    Running {
        pid,
        instance: format!("boot:{pid}"),
        name: name.to_owned(),
        registry_name: None,
        comm: name.chars().take(15).collect(),
    }
}

fn keys(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_owned()).collect()
}

fn observe(
    child: ChildState,
    apps: &[Running],
    before: &[u32],
    wanted: &[&str],
    settled: bool,
) -> Step {
    let before: HashSet<u32> = before.iter().copied().collect();
    step(&Sample {
        child_pid: 100,
        child,
        apps,
        before: &before,
        keys: &keys(wanted),
        settled,
    })
}

#[test]
fn a_live_child_with_no_registry_entry_keeps_polling() {
    let step = observe(ChildState::Running, &[], &[], &["tool"], false);
    assert_eq!(step, Step::Wait);
}

#[test]
fn a_live_child_that_shows_on_the_registry_is_the_owner() {
    let apps = [app(100, "wrapper")];
    let step = observe(ChildState::Running, &apps, &[], &["tool"], false);
    assert_eq!(step, Step::Owner(app(100, "wrapper")));
}

#[test]
fn a_live_child_waits_while_only_an_older_match_is_listed() {
    let apps = [app(7, "tool")];
    assert_eq!(
        observe(ChildState::Running, &apps, &[7], &["tool"], false),
        Step::Wait
    );
}

#[test]
fn a_clean_exit_reports_the_new_owner_not_the_child() {
    let apps = [app(205, "gnome-terminal-server")];
    let step = observe(
        ChildState::Exited(Exit::Code(0)),
        &apps,
        &[],
        &["gnome-terminal-server"],
        false,
    );
    assert_eq!(step, Step::Owner(app(205, "gnome-terminal-server")));
}

#[test]
fn a_clean_exit_after_forwarding_reports_the_running_owner() {
    let apps = [app(7, "gnome-text-editor")];
    let step = observe(
        ChildState::Exited(Exit::Code(0)),
        &apps,
        &[7],
        &["gnome-text-editor"],
        false,
    );
    assert_eq!(step, Step::Owner(app(7, "gnome-text-editor")));
}

#[test]
fn a_new_owner_beats_an_older_one() {
    let apps = [app(7, "gnome-calculator"), app(300, "gnome-calculator")];
    let step = observe(
        ChildState::Exited(Exit::Code(0)),
        &apps,
        &[7],
        &["gnome-calculator"],
        false,
    );
    assert_eq!(step, Step::Owner(app(300, "gnome-calculator")));
}

#[test]
fn two_new_owners_are_a_tie_and_ambiguous() {
    let apps = [app(300, "tool"), app(301, "tool")];
    let step = observe(
        ChildState::Exited(Exit::Code(0)),
        &apps,
        &[],
        &["tool"],
        false,
    );
    assert_eq!(step, Step::Ambiguous(vec![300, 301]));
}

#[test]
fn a_clean_exit_with_no_owner_waits_for_the_registry_then_fails_with_the_status() {
    let exited = ChildState::Exited(Exit::Code(0));
    assert_eq!(observe(exited, &[], &[], &["tool"], false), Step::Wait);
    assert_eq!(
        observe(exited, &[], &[], &["tool"], true),
        Step::Failed(Exit::Code(0))
    );
}

#[test]
fn a_failing_exit_with_no_owner_fails_at_once() {
    let step = observe(
        ChildState::Exited(Exit::Code(3)),
        &[],
        &[],
        &["tool"],
        false,
    );
    assert_eq!(step, Step::Failed(Exit::Code(3)));
}

#[test]
fn a_failing_exit_does_not_adopt_an_older_owner() {
    let apps = [app(7, "tool")];
    let step = observe(
        ChildState::Exited(Exit::Code(2)),
        &apps,
        &[7],
        &["tool"],
        true,
    );
    assert_eq!(step, Step::Failed(Exit::Code(2)));
}

#[test]
fn a_child_killed_by_a_signal_is_a_crash_even_when_something_matches() {
    let apps = [app(300, "tool")];
    let step = observe(
        ChildState::Exited(Exit::Signal(11)),
        &apps,
        &[],
        &["tool"],
        true,
    );
    assert_eq!(step, Step::Failed(Exit::Signal(11)));
}

#[test]
fn a_failure_names_the_status_and_marks_a_crash_in_the_details() {
    let error = failed(Exit::Signal(9), "tool");
    assert_eq!(error.code, ErrorCode::ActionFailed);
    let details = error.details.expect("details");
    assert_eq!(details["kind"], "crashed");
    let exited = failed(Exit::Code(4), "tool");
    assert!(exited.message.contains("status 4"));
}

#[test]
fn a_live_child_that_stayed_a_second_instance_reports_the_new_owner_not_the_old_one() {
    let apps = [app(7, "gnome-calculator"), app(300, "gnome-calculator")];
    let step = observe(
        ChildState::Running,
        &apps,
        &[7],
        &["gnome-calculator"],
        false,
    );
    assert_eq!(step, Step::Owner(app(300, "gnome-calculator")));
}
