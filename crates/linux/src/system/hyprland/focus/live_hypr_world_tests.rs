use super::*;

#[test]
fn the_proof_window_leaves_the_last_read_its_reserve() {
    assert_eq!(
        proof_window(Duration::from_millis(5_000)),
        Duration::from_millis(4_750)
    );
    assert_eq!(proof_window(Duration::from_millis(60_000)), PROOF_BUDGET);
    assert_eq!(proof_window(Duration::from_millis(100)), Duration::ZERO);
}
