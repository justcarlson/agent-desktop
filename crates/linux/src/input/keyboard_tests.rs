use super::*;
use agent_desktop_core::{ErrorCode, Modifier};

fn combo() -> KeyCombo {
    KeyCombo {
        key: "a".to_string(),
        modifiers: vec![Modifier::Ctrl],
    }
}

#[test]
fn an_expired_deadline_fails_before_any_planning_as_not_delivered() {
    let deadline = Deadline::after(1).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(20));
    let map = Keymap::sample();
    for error in [
        combo_plan(&map, &combo(), deadline).unwrap_err(),
        text_plans(&map, "abc", deadline).unwrap_err(),
    ] {
        assert_eq!(error.code, ErrorCode::Timeout);
        assert_eq!(error.details.unwrap()["physical_delivery_started"], false);
    }
}

#[test]
fn a_live_deadline_plans_normally() {
    let deadline = Deadline::after(1_000).unwrap();
    let map = Keymap::sample();
    assert_eq!(combo_plan(&map, &combo(), deadline).unwrap().steps.len(), 4);
    assert_eq!(text_plans(&map, "aa", deadline).unwrap().len(), 1);
}
