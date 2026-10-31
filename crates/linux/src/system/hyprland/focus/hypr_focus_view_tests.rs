use agent_desktop_core::{DeliverySemantics, ErrorCode, FocusSideEffect};

use super::focus_reported;
use super::tests::{
    BYSTANDER, TARGET, compositor_window, frame_window, headed, headless, snapshot, world,
};

#[test]
fn an_active_window_moved_to_a_hidden_special_workspace_is_not_proven_focused() {
    let mut fake = world("special:b", snapshot(TARGET, "1", None, true), None);

    let error = focus_reported(&mut fake, &compositor_window("a"), headless()).unwrap_err();

    assert_eq!(error.code, ErrorCode::PolicyDenied);
    let details = error.details.unwrap();
    assert_eq!(details["cause"], "hidden_special_workspace");
    assert_eq!(details["workspace"], "special:b");
    assert!(fake.activations().is_empty());
}

#[test]
fn headed_focus_reveals_the_hidden_special_workspace_of_a_still_active_window() {
    let mut fake = world(
        "special:b",
        snapshot(TARGET, "1", Some("special:a"), true),
        Some(snapshot(TARGET, "1", Some("special:b"), true)),
    );

    let outcome = focus_reported(&mut fake, &compositor_window("a"), headed()).expect("focused");

    assert_eq!(fake.activations(), vec!["activate 0x10"]);
    assert!(outcome.activation.activated);
    assert_eq!(
        outcome.activation.side_effects,
        vec![
            FocusSideEffect::SpecialWorkspaceClosed {
                monitor: "Virtual-1".into(),
                workspace: "special:a".into(),
            },
            FocusSideEffect::SpecialWorkspaceRevealed {
                monitor: "Virtual-1".into(),
                workspace: "special:b".into(),
            },
        ]
    );
}

#[test]
fn an_active_window_under_an_open_special_workspace_is_refused_before_activation() {
    let mut fake = world("1", snapshot(TARGET, "1", Some("special:a"), true), None);

    let error = focus_reported(&mut fake, &frame_window(), headed()).unwrap_err();

    assert_eq!(error.code, ErrorCode::ActionFailed);
    assert_eq!(error.details.unwrap()["cause"], "special_workspace_open");
    assert!(fake.activations().is_empty());
}

#[test]
fn an_activation_that_leaves_the_window_out_of_view_is_reported_hidden() {
    let mut fake = world(
        "2",
        snapshot(BYSTANDER, "1", None, false),
        Some(snapshot(TARGET, "1", None, true)),
    );

    let error = focus_reported(&mut fake, &frame_window(), headed()).unwrap_err();

    assert_eq!(error.disposition, DeliverySemantics::delivered_unverified());
    let details = error.details.unwrap();
    assert_eq!(details["cause"], "window_hidden");
    assert_eq!(details["in_view"], false);
    assert_eq!(details["activation_sent"], true);
}
