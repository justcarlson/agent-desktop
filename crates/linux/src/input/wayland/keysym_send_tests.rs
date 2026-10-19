use super::super::lead_key;
use super::*;
use crate::input::wayland::fake_session::FakeSession;
use crate::input::wayland::lead_key::LeadPolicy;
use agent_desktop_core::ErrorCode;

const CONTROL_L: u32 = 0xffe3;
const SHIFT_L: u32 = 0xffe1;
const KEY_A: u32 = 0x61;

fn live() -> Deadline {
    Deadline::after(5_000).expect("a five second deadline builds")
}

fn tap(keysym: u32) -> [KeysymStep; 2] {
    [
        KeysymStep { keysym, down: true },
        KeysymStep {
            keysym,
            down: false,
        },
    ]
}

fn ctrl_a() -> Vec<KeysymStep> {
    vec![
        KeysymStep {
            keysym: CONTROL_L,
            down: true,
        },
        KeysymStep {
            keysym: KEY_A,
            down: true,
        },
        KeysymStep {
            keysym: KEY_A,
            down: false,
        },
        KeysymStep {
            keysym: CONTROL_L,
            down: false,
        },
    ]
}

fn drive_with(
    session: &mut FakeSession,
    keys: &[KeysymStep],
    gate: Gate<'_>,
) -> Result<Delivery, AdapterError> {
    run(session, LEAD_POLICY, keys, gate, live())
}

#[test]
fn ctrl_a_runs_create_start_lead_gate_keys_stop_in_that_order() {
    let mut session = FakeSession::new();
    let log = session.log();
    let mut gate = || {
        log.borrow_mut().push("gate".into());
        Ok(())
    };
    let delivery = drive_with(&mut session, &ctrl_a(), &mut gate).expect("ctrl+a delivers");
    let expected = [
        "create",
        "start",
        "down:ffe1",
        "up:ffe1",
        "gate",
        "down:ffe3",
        "down:61",
        "up:61",
        "up:ffe3",
        "stop",
    ];
    assert_eq!(session.events(), expected);
    assert_eq!(delivery.delivered_events, 4);
}

#[test]
fn a_failure_after_create_still_stops_the_session() {
    for label in ["start", "down:ffe1", "down:ffe3", "down:61", "up:ffe3"] {
        let mut session = FakeSession::failing_on(label);
        let result = drive_with(&mut session, &ctrl_a(), &mut || Ok(()));
        assert!(result.is_err(), "{label} must fail the call");
        assert_eq!(session.events().last().map(String::as_str), Some("stop"));
    }
}

#[test]
fn a_failed_gate_sends_no_real_key_stops_and_keeps_its_own_error() {
    let mut session = FakeSession::new();
    let mut gate = || {
        Err(AdapterError::new(
            ErrorCode::ActionFailed,
            "window is not active",
        ))
    };
    let error = drive_with(&mut session, &ctrl_a(), &mut gate).expect_err("the gate refuses");
    assert_eq!(error.message, "window is not active");
    assert_eq!(
        session.events(),
        vec!["create", "start", "down:ffe1", "up:ffe1", "stop"]
    );
}

#[test]
fn a_failed_create_issues_no_stop() {
    let mut session = FakeSession::failing_on("create");
    drive_with(&mut session, &ctrl_a(), &mut || Ok(())).expect_err("create fails");
    assert_eq!(session.events(), vec!["create"]);
}

#[test]
fn a_deadline_spent_before_start_times_out_and_still_stops() {
    let mut session = FakeSession::new();
    let expired = Deadline::after(0).expect("a zero deadline builds");
    let error = run(
        &mut session,
        LEAD_POLICY,
        &ctrl_a(),
        &mut || Ok(()),
        expired,
    )
    .expect_err("an expired deadline fails");
    assert_eq!(error.code, ErrorCode::Timeout);
    assert_eq!(session.events(), vec!["create", "stop"]);
}

#[test]
fn a_session_that_drops_its_first_key_loses_only_the_lead() {
    let mut session = FakeSession::new();
    session.drops_first_key = true;
    drive_with(&mut session, &ctrl_a(), &mut || Ok(())).expect("the keys deliver");
    let real: Vec<(u32, bool)> = ctrl_a().iter().map(|k| (k.keysym, k.down)).collect();
    assert_eq!(
        &session.arrived[session.arrived.len() - 4..],
        real.as_slice()
    );
    assert_eq!(session.arrived.len(), 5);
}

#[test]
fn without_a_lead_the_dropped_first_key_would_be_a_real_one() {
    let mut session = FakeSession::new();
    session.drops_first_key = true;
    let none = LeadPolicy {
        tap: None,
        settle: std::time::Duration::ZERO,
    };
    run(&mut session, none, &ctrl_a(), &mut || Ok(()), live()).expect("delivers");
    assert_eq!(session.arrived.len(), 3);
}

#[test]
fn keys_held_when_a_call_fails_are_released_before_the_stop() {
    let mut session = FakeSession::failing_on("up:61");
    drive_with(&mut session, &ctrl_a(), &mut || Ok(())).expect_err("the release fails");
    let events = session.events();
    let tail: Vec<&str> = events.iter().rev().take(3).map(String::as_str).collect();
    assert_eq!(tail, vec!["stop", "up:ffe3", "up:61"]);
}

#[test]
fn a_failure_after_keys_went_out_is_uncertain_and_before_any_is_not_delivered() {
    let mut late = FakeSession::failing_on("down:61");
    let after = drive_with(&mut late, &ctrl_a(), &mut || Ok(())).expect_err("fails");
    assert_eq!(after.disposition, DeliverySemantics::uncertain());
    assert_eq!(
        after.details.expect("details")["physical_delivery_started"],
        true
    );

    let mut early = FakeSession::failing_on("down:ffe3");
    let before = drive_with(&mut early, &ctrl_a(), &mut || Ok(())).expect_err("fails");
    assert_eq!(before.disposition, DeliverySemantics::not_delivered());
    assert_eq!(
        before.details.expect("details")["physical_delivery_started"],
        false
    );
}

#[test]
fn a_text_of_wide_characters_goes_as_keysyms_and_no_error_repeats_it() {
    let keys: Vec<KeysymStep> = "你好"
        .chars()
        .flat_map(|c| tap(0x0100_0000 | u32::from(c)))
        .collect();
    let mut session = FakeSession::new();
    let delivery = drive_with(&mut session, &keys, &mut || Ok(())).expect("delivers");
    assert_eq!(delivery.delivered_events, 4);

    let mut failing = FakeSession::failing_on("down:1004f60");
    let error = drive_with(&mut failing, &keys, &mut || Ok(())).expect_err("fails");
    let rendered = format!("{error:?}");
    for leak in ["你", "好", "4f60", "20320"] {
        assert!(!rendered.contains(leak), "the error repeated {leak}");
    }
}

#[test]
fn no_keys_means_no_session_at_all() {
    let delivery = send_keys(&[], &mut || Ok(()), live()).expect("nothing to send");
    assert_eq!(delivery.delivered_events, 0);
}

#[test]
fn a_pointer_command_is_refused_with_the_semantic_suggestion_and_nothing_sent() {
    let error = pointer_refusal("mouse_event");
    assert_eq!(error.code, ErrorCode::ActionNotSupported);
    assert!(error.message.contains("mouse_event"));
    assert!(
        error
            .suggestion
            .is_some_and(|text| text.contains("semantic action"))
    );
    assert_eq!(
        error.details.expect("details")["physical_delivery_started"],
        false
    );
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
}

#[test]
fn the_lead_is_a_shift_tap() {
    assert_eq!(
        lead_key::lead_steps(LEAD_POLICY)
            .iter()
            .map(|s| s.keysym)
            .collect::<Vec<_>>(),
        vec![SHIFT_L, SHIFT_L]
    );
}

#[test]
fn the_session_is_held_open_while_the_compositor_drains_the_last_keys() {
    let mut session = FakeSession::new();
    let started = std::time::Instant::now();
    drive_with(&mut session, &ctrl_a(), &mut || Ok(())).expect("delivers");
    assert!(started.elapsed() >= DRAIN);
}

#[test]
fn a_failed_delivery_does_not_wait_to_drain() {
    let mut session = FakeSession::failing_on("down:ffe3");
    let started = std::time::Instant::now();
    drive_with(&mut session, &ctrl_a(), &mut || Ok(())).expect_err("fails");
    assert!(started.elapsed() < DRAIN);
}

#[test]
fn text_outside_the_layout_is_refused_before_a_session_is_opened() {
    let keys: Vec<KeysymStep> = "你好"
        .chars()
        .flat_map(|c| tap(0x0100_0000 | u32::from(c)))
        .collect();
    let error = send_keys(&keys, &mut || Ok(()), live()).expect_err("refused");
    assert_eq!(error.code, ErrorCode::ActionNotSupported);
    assert_eq!(
        error.details.expect("details")["cause"],
        "character_outside_layout"
    );
}
