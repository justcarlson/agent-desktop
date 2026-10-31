use super::*;

fn states(values: &[u32]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_ne_bytes())
        .collect()
}

#[test]
fn a_state_array_holding_two_is_activated() {
    assert!(holds_activated(&states(&[0, 2])));
    assert!(holds_activated(&states(&[2])));
}

#[test]
fn a_state_array_without_two_or_with_a_partial_value_is_not_activated() {
    assert!(!holds_activated(&states(&[])));
    assert!(!holds_activated(&states(&[0, 1, 3])));
    assert!(!holds_activated(&[2, 0, 0]));
}
