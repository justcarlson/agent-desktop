use std::time::Duration;

use super::super::fake_session::FakeSession;
use super::*;

const MODIFIER_KEYSYMS: std::ops::RangeInclusive<u32> = 0xffe1..=0xffee;

fn live() -> Deadline {
    Deadline::after(5_000).expect("a five second deadline builds")
}

fn settle_only(millis: u64) -> LeadPolicy {
    LeadPolicy {
        tap: None,
        settle: Duration::from_millis(millis),
    }
}

#[test]
fn the_chosen_lead_is_a_modifier_tap_and_never_a_printable_key() {
    let steps = lead_steps(LEAD_POLICY);
    assert!(!steps.is_empty());
    assert!(
        steps
            .iter()
            .all(|step| MODIFIER_KEYSYMS.contains(&step.keysym))
    );
}

#[test]
fn a_modifier_tap_presses_then_releases_the_same_key() {
    let steps = lead_steps(LeadPolicy {
        tap: Some(0xffe3),
        settle: Duration::ZERO,
    });
    assert_eq!(
        steps,
        vec![
            KeysymStep {
                keysym: 0xffe3,
                down: true
            },
            KeysymStep {
                keysym: 0xffe3,
                down: false
            },
        ]
    );
}

#[test]
fn a_settle_lead_sends_no_key_and_waits() {
    let policy = settle_only(40);
    assert!(lead_steps(policy).is_empty());
    assert_eq!(policy.settle, Duration::from_millis(40));
}

#[test]
fn the_lead_reaches_the_session_as_key_calls() {
    let mut session = FakeSession::new();
    send_lead(&mut session, LEAD_POLICY, live()).expect("the lead sends");
    assert_eq!(session.events().len(), 2);
}

#[test]
fn a_settle_waits_before_the_real_keys_can_follow() {
    let mut session = FakeSession::new();
    let started = std::time::Instant::now();
    send_lead(&mut session, settle_only(60), live()).expect("the settle completes");
    assert!(started.elapsed() >= Duration::from_millis(60));
    assert!(session.events().is_empty());
}

#[test]
fn the_chosen_lead_waits_for_the_compositor_to_attach_the_keyboard() {
    assert!(LEAD_POLICY.settle >= Duration::from_millis(50));
}
