use std::collections::HashMap;

use agent_desktop_core::Deadline;

use super::walker::walk_levels;
use super::{Walk, WalkLimits, WalkedNode};

type Graph = HashMap<&'static str, Vec<&'static str>>;

fn walk(graph: &Graph, limits: &WalkLimits) -> Walk<&'static str> {
    let deadline = Deadline::after(5_000).expect("a five second deadline builds");
    walk_levels("a", limits, deadline, |level| {
        Ok(level
            .iter()
            .map(|key| (*key, graph.get(key).cloned().unwrap_or_default()))
            .collect())
    })
    .expect("an in-memory walk cannot fail")
}

fn names(node: &WalkedNode<&'static str>) -> Vec<&'static str> {
    let mut found = vec![node.payload];
    for child in &node.children {
        found.extend(names(child));
    }
    found
}

#[test]
fn a_child_that_points_back_to_an_ancestor_is_skipped_and_the_walk_completes() {
    let graph = Graph::from([("a", vec!["b"]), ("b", vec!["a", "c"])]);
    let result = walk(&graph, &WalkLimits::default());
    assert!(result.complete);
    assert_eq!(names(&result.root), ["a", "b", "c"]);
}

#[test]
fn a_node_reached_through_two_parents_is_visited_under_both() {
    let graph = Graph::from([("a", vec!["b", "c"]), ("b", vec!["d"]), ("c", vec!["d"])]);
    let result = walk(&graph, &WalkLimits::default());
    assert!(result.complete);
    assert_eq!(names(&result.root), ["a", "b", "d", "c", "d"]);
    assert_eq!(result.node_count, 5);
}

#[test]
fn a_node_budget_smaller_than_the_tree_marks_the_cut_nodes_and_the_result_incomplete() {
    let graph = Graph::from([("a", vec!["b", "c", "d"])]);
    let limits = WalkLimits {
        max_depth: 50,
        max_nodes: 3,
    };
    let result = walk(&graph, &limits);
    assert!(!result.complete);
    assert!(result.root.truncated);
    assert_eq!(names(&result.root), ["a", "b", "c"]);
    assert!(result.root.children.iter().all(|child| !child.truncated));
}
