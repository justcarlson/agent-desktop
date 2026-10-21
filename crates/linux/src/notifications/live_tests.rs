use agent_desktop_core::{
    Deadline, DismissNotificationRequest, InteractionLease, InteractionPolicy, NotificationFilter,
    NotificationIdentity,
};
use std::process::Command;
use std::time::Duration;

use super::signals::{BusSignals, Signal, SignalSource};
use crate::system::SharedBus;

fn oracle(args: &[&str]) -> String {
    let script = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../scripts/notification-oracle.sh"
    );
    let out = Command::new("bash")
        .arg(script)
        .args(args)
        .output()
        .expect("oracle");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

fn notify(app: &str, summary: &str, actions: &str) -> u32 {
    oracle(&["notify", app, summary, "body", actions])
        .parse()
        .expect("id")
}

fn lease() -> InteractionLease {
    InteractionLease::guarded(Deadline::after(60_000).expect("deadline"), ()).expect("lease")
}

fn deadline() -> Deadline {
    Deadline::after(30_000).expect("deadline")
}

fn listed(bus: &SharedBus) -> Vec<agent_desktop_core::NotificationInfo> {
    super::list(
        bus,
        &NotificationFilter::default(),
        InteractionPolicy::headless(),
        deadline(),
        None,
    )
    .expect("listed")
}

fn identity_of(info: &agent_desktop_core::NotificationInfo) -> NotificationIdentity {
    NotificationIdentity {
        expected_app: Some(info.app_name.clone()),
        expected_title: Some(info.title.clone()),
    }
}

#[test]
#[ignore = "needs the private gnome-shell of scripts/notification-live.sh"]
fn signals_subscribed_before_a_close_arrive_and_after_it_do_not() {
    let early = BusSignals.subscribe(deadline()).expect("subscribed");
    let id = notify("live-signals", "closed early", "");
    oracle(&["close", &id.to_string()]);
    let seen = early.collect(Duration::from_secs(3));
    assert!(seen.contains(&Signal::Closed(id)), "{seen:?}");

    let id = notify("live-signals", "closed before the rule", "");
    oracle(&["close", &id.to_string()]);
    let late = BusSignals.subscribe(deadline()).expect("subscribed");
    assert!(late.collect(Duration::from_secs(2)).is_empty());
}

#[test]
#[ignore = "needs the private gnome-shell of scripts/notification-live.sh"]
fn list_matches_what_the_sender_posted() {
    let bus = SharedBus::default();
    notify("live-mail", "first", "k1=Run A");
    notify("live-chat", "second", "");
    let entries = listed(&bus);
    assert_eq!(entries[0].title, "second");
    assert_eq!(entries[1].title, "first");
    assert_eq!(entries[1].actions, vec!["Run A"]);
}

#[test]
#[ignore = "needs the private gnome-shell of scripts/notification-live.sh"]
fn an_entry_that_has_gone_is_not_found() {
    let bus = SharedBus::default();
    let id = notify("live-gone", "vanishing", "");
    let info = listed(&bus).remove(0);
    let who = identity_of(&info);
    oracle(&["close", &id.to_string()]);
    let request = DismissNotificationRequest {
        index: info.index,
        app_filter: None,
        identity: &who,
        policy: InteractionPolicy::headed(),
    };
    let error = super::dismiss(&bus, request, &lease()).expect_err("gone");
    assert_eq!(
        error.code,
        agent_desktop_core::ErrorCode::NotificationNotFound
    );
}
