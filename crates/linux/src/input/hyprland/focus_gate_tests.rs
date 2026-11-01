use std::cell::RefCell;

use agent_desktop_core::ErrorCode;

use super::*;

fn gate(
    reads: Vec<Option<u64>>,
    expected: Option<u64>,
) -> FocusGate<impl FnMut() -> Result<Option<u64>, AdapterError>> {
    let reads = RefCell::new(reads.into_iter());
    FocusGate::new(
        move || {
            reads
                .borrow_mut()
                .next()
                .ok_or_else(|| AdapterError::new(ErrorCode::Internal, "no read"))
        },
        expected,
    )
}

fn outcomes(
    mut gate: FocusGate<impl FnMut() -> Result<Option<u64>, AdapterError>>,
    n: usize,
) -> Vec<bool> {
    (0..n).map(|_| gate.check().is_ok()).collect()
}

#[test]
fn a_steady_focus_passes_every_check() {
    assert_eq!(outcomes(gate(vec![Some(1); 4], None), 4), vec![true; 4]);
}

#[test]
fn the_first_press_may_move_focus_once() {
    let reads = vec![Some(1), Some(2), Some(2), Some(2)];
    assert_eq!(outcomes(gate(reads, None), 4), vec![true; 4]);
}

#[test]
fn a_later_change_refuses() {
    let reads = vec![Some(1), Some(2), Some(2), Some(3)];
    assert_eq!(
        outcomes(gate(reads, None), 4),
        vec![true, true, true, false]
    );
    let reads = vec![Some(1), Some(1), None];
    assert_eq!(outcomes(gate(reads, None), 3), vec![true, true, false]);
}

#[test]
fn the_rebaseline_must_land_on_the_expected_window() {
    let reads = vec![Some(1), Some(3)];
    assert_eq!(outcomes(gate(reads, Some(2)), 2), vec![true, false]);
    let reads = vec![Some(1), Some(2), Some(2)];
    assert_eq!(outcomes(gate(reads, Some(2)), 3), vec![true; 3]);
}

#[test]
fn a_change_refusal_names_the_cause() {
    let mut gate = gate(vec![Some(1), Some(1), Some(4)], None);
    let _ = gate.check();
    let _ = gate.check();
    let details = gate
        .check()
        .err()
        .and_then(|e| e.details)
        .unwrap_or_default();
    assert_eq!(details["cause"], "focus_changed");
}

#[test]
fn an_unreadable_focus_refuses() {
    let mut gate = gate(vec![], None);
    assert!(gate.check().is_err());
}
