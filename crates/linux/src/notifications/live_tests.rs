use agent_desktop_core::{
    AdapterError, Deadline, DeliverySemantics, DismissAllNotificationsRequest,
    DismissNotificationRequest, ErrorCode, InteractionLease, InteractionPolicy,
    NotificationActionRequest, NotificationFilter, NotificationIdentity, SystemOps,
};

use super::refusal_oracle::Oracle;
use crate::system::SharedBus;

fn deadline() -> Deadline {
    Deadline::after(30_000).expect("deadline")
}

fn lease() -> InteractionLease {
    crate::LinuxAdapter::new()
        .acquire_interaction_lease(deadline())
        .expect("lease")
}

fn refused<T: std::fmt::Debug>(result: Result<T, AdapterError>) {
    let error = result.expect_err("unproven notification semantics must refuse");
    assert_eq!(error.code, ErrorCode::ActionNotSupported, "{error:?}");
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
    assert_eq!(
        error.details.as_ref().unwrap()["kind"],
        "notification_identity_unavailable"
    );
    assert_eq!(error.details.as_ref().unwrap()["retryable"], false);
}

#[test]
#[ignore = "requires a private GNOME notification session"]
fn label_positions_do_not_prove_protocol_fields() {
    let mut oracle = Oracle::start();
    for (app, title, body) in [
        ("البريد", "Résumé 🎉", "שלום"),
        ("Second Client", "body", "app"),
    ] {
        oracle.notify(app, title, body);
        refused(super::list(
            &SharedBus::default(),
            &NotificationFilter::default(),
            InteractionPolicy::headless(),
            deadline(),
            None,
        ));
    }
    oracle.assert_untouched();
}

#[test]
#[ignore = "requires a private GNOME notification session"]
fn time_phrases_do_not_authorize_dropping_notification_text() {
    let mut oracle = Oracle::start();
    for title in ["5 minutes ago", "昨日"] {
        oracle.notify("Clock Sender", title, "actual body");
        refused(super::list(
            &SharedBus::default(),
            &NotificationFilter::default(),
            InteractionPolicy::headless(),
            deadline(),
            None,
        ));
    }
    oracle.assert_untouched();
}

#[test]
#[ignore = "requires a private GNOME notification session"]
fn unnamed_button_ordinals_do_not_authorize_dismissal() {
    let mut oracle = Oracle::start();
    oracle.notify("Control Sender", "Target", "payload");
    let identity = NotificationIdentity {
        expected_app: Some("Control Sender".into()),
        expected_title: Some("Target".into()),
    };
    let policy = InteractionPolicy::headed();
    refused(super::dismiss(
        &SharedBus::default(),
        DismissNotificationRequest {
            index: 1,
            app_filter: None,
            identity: &identity,
            policy,
        },
        &lease(),
    ));
    oracle.assert_untouched();
}

#[test]
#[ignore = "requires a private GNOME notification session"]
fn an_action_label_does_not_prove_a_protocol_action_or_expand_control() {
    let mut oracle = Oracle::start();
    oracle.notify("Action Sender", "Target", "payload");
    let identity = NotificationIdentity {
        expected_app: Some("Action Sender".into()),
        expected_title: Some("Target".into()),
    };
    refused(super::action(
        &SharedBus::default(),
        NotificationActionRequest {
            index: 1,
            identity: &identity,
            action_name: "Ouvrir",
            policy: InteractionPolicy::headed(),
        },
        &lease(),
    ));
    oracle.assert_untouched();
}

#[test]
#[ignore = "requires a private GNOME notification session"]
fn notification_wait_preflight_does_not_advertise_unproven_identity() {
    let oracle = Oracle::start();
    refused(crate::LinuxAdapter::new().check_notification_support(deadline()));
    oracle.assert_untouched();
}

#[test]
#[ignore = "requires a private GNOME notification session"]
fn a_footer_button_does_not_prove_clear_all() {
    let mut oracle = Oracle::start();
    oracle.notify("Clear Sender", "Target", "payload");
    oracle.notify("Bystander Sender", "Keep", "bystander");
    for app_filter in [None, Some("Clear Sender")] {
        refused(super::dismiss_all(
            &SharedBus::default(),
            DismissAllNotificationsRequest {
                app_filter,
                policy: InteractionPolicy::headed(),
            },
            &lease(),
        ));
    }
    oracle.assert_untouched();
}

#[test]
#[ignore = "requires a private GNOME notification session"]
fn an_empty_projection_does_not_prove_an_empty_notification_history() {
    let oracle = Oracle::start();
    refused(super::list(
        &SharedBus::default(),
        &NotificationFilter::default(),
        InteractionPolicy::headless(),
        deadline(),
        None,
    ));
    oracle.assert_untouched();
}
