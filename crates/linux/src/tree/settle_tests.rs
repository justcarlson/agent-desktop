use std::time::Duration;

use super::*;
use crate::tree::role_numbers::{DOCUMENT_FRAME, DOCUMENT_WEB, PANEL};
use crate::tree::test_support::read_of;

const MS: fn(u64) -> Duration = Duration::from_millis;

fn run(curve: &[usize], remaining: Duration) -> Vec<Step> {
    let mut settle = Settle::new(remaining);
    let mut steps = Vec::new();
    let mut waited = Duration::ZERO;
    for nodes in curve {
        let step = settle.after_walk(*nodes, waited);
        steps.push(step);
        match step {
            Step::Wait(gap) => waited += gap,
            Step::Settled | Step::GiveUp => break,
        }
    }
    steps
}

#[test]
fn a_plateau_at_the_empty_shell_is_not_a_settled_tree() {
    let steps = run(&[18, 18, 18, 18, 370, 380, 386, 386, 386], MS(60_000));
    assert_eq!(steps.len(), 9);
    assert_eq!(steps[3], Step::Wait(POLL));
    assert_eq!(steps[8], Step::Settled);
}

#[test]
fn a_filling_tree_settles_on_the_third_equal_read() {
    let steps = run(&[66, 120, 226, 247, 247, 247, 300], MS(60_000));
    assert_eq!(steps.len(), 6);
    assert_eq!(steps[5], Step::Settled);
}

#[test]
fn two_equal_reads_above_the_ceiling_are_not_enough() {
    let steps = run(&[247, 247], MS(60_000));
    assert_eq!(steps, vec![Step::Wait(POLL), Step::Wait(POLL)]);
}

#[test]
fn a_budget_smaller_than_one_poll_gives_up_at_once() {
    assert_eq!(run(&[20, 20], MS(200)), vec![Step::GiveUp]);
}

#[test]
fn the_wait_stops_at_its_share_of_the_budget() {
    let steps = run(&[20; 40], MS(1_000));
    assert_eq!(steps.last(), Some(&Step::GiveUp));
    let waits = steps
        .iter()
        .filter(|step| matches!(step, Step::Wait(_)))
        .count();
    assert_eq!(waits, 3);
}

#[test]
fn a_tree_that_settles_on_the_last_affordable_poll_is_settled_not_given_up() {
    let steps = run(&[100, 100, 100], MS(1_000));
    assert_eq!(steps.last(), Some(&Step::Settled));
}

fn node(role: u32, children: Vec<WalkedNode<ObservedRead>>) -> WalkedNode<ObservedRead> {
    WalkedNode {
        payload: read_of(role, "n", &[]),
        children,
        truncated: false,
        child_count: None,
        source_index: 0,
        in_web_document: false,
    }
}

#[test]
fn a_web_document_below_the_root_marks_a_web_tree() {
    let tree = node(PANEL, vec![node(PANEL, vec![node(DOCUMENT_WEB, vec![])])]);
    assert!(has_web_document(&tree));
}

#[test]
fn a_native_document_frame_is_not_a_web_document() {
    let tree = node(PANEL, vec![node(DOCUMENT_FRAME, vec![])]);
    assert!(!has_web_document(&tree));
}
