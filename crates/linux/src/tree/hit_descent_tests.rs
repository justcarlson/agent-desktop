use std::collections::HashMap;
use std::time::Duration;

use super::*;

pub(crate) fn el(path: &str) -> AtspiElement {
    AtspiElement::new(":1.1", path, 1)
}

/// In-memory nodes: where each object's point query leads, each object's
/// verified parent, and what each describes as. Every call is recorded.
#[derive(Default)]
pub(crate) struct FakeNodes {
    pub(crate) leads: HashMap<String, Hit>,
    pub(crate) parents: HashMap<String, Option<String>>,
    pub(crate) described: HashMap<String, Described>,
    pub(crate) endless: bool,
    pub(crate) points: Vec<(i32, i32)>,
    pub(crate) calls: Vec<&'static str>,
}

impl FakeNodes {
    pub(crate) fn lead(&mut self, from: &str, to: &str) -> &mut Self {
        self.leads.insert(from.to_owned(), Hit::Object(el(to)));
        self
    }

    pub(crate) fn parent(&mut self, node: &str, parent: &str) -> &mut Self {
        self.parents
            .insert(node.to_owned(), Some(parent.to_owned()));
        self
    }
}

impl NodeSource for FakeNodes {
    fn at_point(&mut self, from: &AtspiElement, point: (i32, i32)) -> Result<Hit, AdapterError> {
        self.calls.push("at_point");
        self.points.push(point);
        if self.endless {
            return Ok(Hit::Object(el(&format!("{}x", from.path()))));
        }
        Ok(self.leads.get(from.path()).cloned().unwrap_or(Hit::Null))
    }

    fn verified_parent(
        &mut self,
        node: &AtspiElement,
    ) -> Result<Option<AtspiElement>, AdapterError> {
        self.calls.push("verified_parent");
        Ok(self
            .parents
            .get(node.path())
            .cloned()
            .flatten()
            .map(|path| el(&path)))
    }

    fn describe(&mut self, node: &AtspiElement) -> Result<Described, AdapterError> {
        self.calls.push("describe");
        Ok(self.described.get(node.path()).cloned().unwrap_or_default())
    }
}

fn deadline() -> Deadline {
    Deadline::after(5_000).expect("a deadline builds")
}

fn tree() -> FakeNodes {
    let mut nodes = FakeNodes::default();
    nodes
        .parent("/pane", "/frame")
        .parent("/target", "/pane")
        .parent("/child", "/target")
        .parent("/other", "/frame")
        .parent("/deep", "/target");
    nodes
}

fn run(nodes: &mut FakeNodes) -> Located {
    let frame = el("/frame");
    let descent = descend(nodes, &frame, (10, 10), deadline()).expect("the descent runs");
    locate(
        nodes,
        &el("/target"),
        &frame,
        &descent,
        (10, 10),
        deadline(),
    )
    .expect("locate runs")
}

#[test]
fn a_chain_ending_at_the_target_lands_on_it() {
    let mut nodes = tree();
    nodes
        .lead("/frame", "/pane")
        .lead("/pane", "/target")
        .lead("/target", "/target");
    assert_eq!(run(&mut nodes).landing, Landing::OnTarget);
}

#[test]
fn a_chain_ending_one_level_below_the_target_lands_on_it() {
    let mut nodes = tree();
    nodes
        .lead("/frame", "/pane")
        .lead("/pane", "/target")
        .lead("/target", "/child");
    assert_eq!(run(&mut nodes).landing, Landing::OnTarget);
}

#[test]
fn a_landing_that_skips_the_target_but_sits_under_it_is_on_the_target() {
    let mut nodes = tree();
    nodes.lead("/frame", "/deep");
    assert_eq!(run(&mut nodes).landing, Landing::OnTarget);
}

#[test]
fn a_chain_ending_at_the_targets_parent_is_an_ancestor() {
    let mut nodes = tree();
    nodes.lead("/frame", "/pane");
    assert_eq!(run(&mut nodes).landing, Landing::Ancestor);
}

#[test]
fn a_chain_ending_at_the_window_itself_is_an_ancestor() {
    let mut nodes = tree();
    nodes.lead("/frame", "/frame");
    assert_eq!(run(&mut nodes).landing, Landing::Ancestor);
}

#[test]
fn an_unrelated_node_is_elsewhere_with_what_it_describes() {
    let mut nodes = tree();
    nodes.lead("/frame", "/other");
    nodes.described.insert(
        "/other".to_owned(),
        Described {
            role: Some("push button".to_owned()),
            name: Some("Close".to_owned()),
            window_extents: Some(Extents {
                x: 0,
                y: 0,
                width: 50,
                height: 50,
            }),
        },
    );
    let located = run(&mut nodes);
    assert_eq!(located.landing, Landing::Elsewhere { contained: true });
    assert_eq!(
        located.lander.expect("described").name.as_deref(),
        Some("Close")
    );
}

#[test]
fn an_unrelated_node_whose_extents_miss_the_point_is_not_contained() {
    let mut nodes = tree();
    nodes.lead("/frame", "/other");
    nodes.described.insert(
        "/other".to_owned(),
        Described {
            window_extents: Some(Extents {
                x: 400,
                y: 400,
                width: 50,
                height: 50,
            }),
            ..Described::default()
        },
    );
    assert_eq!(
        run(&mut nodes).landing,
        Landing::Elsewhere { contained: false }
    );
}

#[test]
fn a_climb_step_that_fails_verification_leaves_the_landing_unresolved() {
    let mut nodes = tree();
    nodes.lead("/frame", "/other");
    nodes.parents.insert("/other".to_owned(), None);
    assert_eq!(run(&mut nodes).landing, Landing::Unresolved);
    let mut nodes = tree();
    nodes.lead("/frame", "/other");
    nodes.parents.insert("/target".to_owned(), None);
    assert_eq!(run(&mut nodes).landing, Landing::Unresolved);
}

#[test]
fn a_descent_that_never_reaches_null_stops_at_the_cap() {
    let mut nodes = tree();
    nodes.endless = true;
    let frame = el("/frame");
    let descent = descend(&mut nodes, &frame, (1, 1), deadline()).expect("runs");
    assert_eq!(nodes.points.len(), 50);
    let located = locate(
        &mut nodes,
        &el("/target"),
        &frame,
        &descent,
        (1, 1),
        deadline(),
    )
    .expect("locate runs");
    assert_eq!(located.landing, Landing::Unresolved);
}

#[test]
fn null_at_the_window_and_a_failed_query_are_unresolved() {
    let mut nodes = tree();
    assert_eq!(run(&mut nodes).landing, Landing::Unresolved);
    let mut nodes = tree();
    nodes.leads.insert("/frame".to_owned(), Hit::Failed);
    assert_eq!(run(&mut nodes).landing, Landing::Unresolved);
}

#[test]
fn an_expired_deadline_is_the_timeout_and_not_a_landing() {
    let mut nodes = tree();
    nodes.lead("/frame", "/target");
    let spent = Deadline::after(1).expect("a deadline builds");
    std::thread::sleep(Duration::from_millis(20));
    let error = descend(&mut nodes, &el("/frame"), (1, 1), spent).expect_err("timed out");
    assert_eq!(error.code, agent_desktop_core::ErrorCode::Timeout);
    assert!(nodes.calls.is_empty());
}

#[test]
fn only_the_point_query_the_climb_and_the_description_are_ever_read() {
    let mut nodes = tree();
    nodes.lead("/frame", "/other");
    run(&mut nodes);
    assert!(
        nodes
            .calls
            .iter()
            .all(|call| ["at_point", "verified_parent", "describe"].contains(call))
    );
    assert!(nodes.calls.contains(&"at_point"));
}

#[test]
fn the_descent_starts_at_the_window_with_the_window_point() {
    let mut nodes = tree();
    nodes.lead("/frame", "/target");
    descend(&mut nodes, &el("/frame"), (210, 17), deadline()).expect("runs");
    assert_eq!(nodes.points.first(), Some(&(210, 17)));
}
