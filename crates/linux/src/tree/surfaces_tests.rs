use super::is_sheet;
use crate::tree::test_support::{read_of, words};
use crate::tree::{Field, RawNode};

fn node(role: u32, state_bits: &[u32]) -> RawNode {
    let mut node = read_of(role, "d", &[]).node;
    node.traits.states = Field::Present(words(state_bits));
    node
}

#[test]
fn a_modal_dialog_is_a_sheet() {
    assert!(is_sheet(&node(16, &[25, 16])));
}

#[test]
fn a_modal_frame_and_a_modeless_dialog_are_not_sheets() {
    assert!(!is_sheet(&node(23, &[25, 16])));
    assert!(!is_sheet(&node(16, &[25])));
}

#[test]
fn an_unreadable_role_or_state_set_is_never_a_sheet() {
    let mut failed_role = node(16, &[16]);
    failed_role.traits.role = Field::Failed("role".to_owned());
    assert!(!is_sheet(&failed_role));
    let mut failed_states = node(16, &[]);
    failed_states.traits.states = Field::Failed("states".to_owned());
    assert!(!is_sheet(&failed_states));
}
