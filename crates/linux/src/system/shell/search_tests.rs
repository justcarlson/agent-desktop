use agent_desktop_core::Deadline;

use crate::system::shell::fake_tree::{FakeTree, gnome_shell};
use crate::system::shell::light::Light;
use crate::system::shell::search::{Query, find};

fn deadline() -> Deadline {
    Deadline::after(10_000).expect("a deadline builds")
}

fn named<'a>(name: &'a str) -> impl Fn(&Light) -> bool + 'a {
    move |light| light.name == name
}

fn anywhere(_: &[Light], _: &Light) -> bool {
    true
}

#[test]
fn the_chain_runs_from_the_root_to_the_match_in_order() {
    let tree = gnome_shell(true, false);
    let candidate = named("Activities");
    let path = find(
        &tree,
        &crate::system::shell::fake_tree::element("/w"),
        deadline(),
        &Query {
            prune: &|_| false,
            candidate: &candidate,
            accept: &anywhere,
        },
    )
    .expect("the search runs")
    .expect("the node exists");
    let paths: Vec<&str> = path.iter().map(|light| light.element.path()).collect();
    assert_eq!(paths, ["/w", "/p2", "/top", "/bar", "/left", "/activities"]);
}

#[test]
fn a_pruned_subtree_is_neither_entered_nor_matched() {
    let tree = gnome_shell(true, false);
    let candidate = named("Show Apps");
    let prune = |light: &Light| light.name == "Overview";
    let found = find(
        &tree,
        &crate::system::shell::fake_tree::element("/w"),
        deadline(),
        &Query {
            prune: &prune,
            candidate: &candidate,
            accept: &anywhere,
        },
    )
    .expect("the search runs");
    assert!(found.is_none());
}

#[test]
fn a_node_that_vanishes_during_the_search_is_skipped() {
    let mut tree = gnome_shell(true, false);
    tree.vanish("/left");
    let candidate = named("System");
    let found = find(
        &tree,
        &crate::system::shell::fake_tree::element("/w"),
        deadline(),
        &Query {
            prune: &|_| false,
            candidate: &candidate,
            accept: &anywhere,
        },
    )
    .expect("a vanished node is not an error");
    assert!(found.is_some());
}

#[test]
fn the_acceptance_rule_sees_the_ancestors_and_can_refuse() {
    let tree = gnome_shell(true, false);
    let candidate = named("System");
    let refuse = |above: &[Light], _: &Light| above.iter().any(|node| node.name == "nowhere");
    let found = find(
        &tree,
        &crate::system::shell::fake_tree::element("/w"),
        deadline(),
        &Query {
            prune: &|_| false,
            candidate: &candidate,
            accept: &refuse,
        },
    )
    .expect("the search runs");
    assert!(found.is_none());
}

#[test]
fn an_expired_deadline_is_a_timeout_not_an_empty_result() {
    let tree = FakeTree::new();
    let expired = Deadline::after(1).expect("a deadline builds");
    std::thread::sleep(std::time::Duration::from_millis(5));
    let error = find(
        &tree,
        &crate::system::shell::fake_tree::element("/w"),
        expired,
        &Query {
            prune: &|_| false,
            candidate: &|_| true,
            accept: &anywhere,
        },
    )
    .expect_err("no time is no answer");
    assert_eq!(error.code, agent_desktop_core::ErrorCode::Timeout);
}
