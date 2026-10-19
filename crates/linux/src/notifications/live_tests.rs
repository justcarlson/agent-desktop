use std::process::Command;
use std::time::Duration;

use agent_desktop_core::{
    Deadline, DismissAllNotificationsRequest, DismissNotificationRequest, InteractionLease,
    InteractionPolicy, NotificationActionRequest, NotificationFilter, NotificationIdentity,
};

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

struct Monitor {
    pid: String,
    file: std::path::PathBuf,
}

impl Monitor {
    fn start(name: &str) -> Self {
        let file = std::env::temp_dir().join(format!("3.14-c-{name}-{}.log", std::process::id()));
        let pid = oracle(&["monitor", file.to_str().expect("path")]);
        std::thread::sleep(Duration::from_millis(500));
        Self { pid, file }
    }

    fn text(&self) -> String {
        std::thread::sleep(Duration::from_secs(1));
        std::fs::read_to_string(&self.file).unwrap_or_default()
    }
}

impl Drop for Monitor {
    fn drop(&mut self) {
        let _ = Command::new("kill").arg(&self.pid).status();
        let _ = std::fs::remove_file(&self.file);
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
#[ignore = "needs the private gnome-shell and lane B's real shell core"]
fn needs_shell_core_list_matches_what_the_sender_posted() {
    let bus = SharedBus::default();
    notify("live-mail", "first", "k1=Run A");
    notify("live-chat", "second", "");
    let entries = listed(&bus);
    assert_eq!(entries[0].title, "second");
    assert_eq!(entries[1].title, "first");
    assert_eq!(entries[1].actions, vec!["Run A"]);
}

#[test]
#[ignore = "needs the private gnome-shell and lane B's real shell core"]
fn needs_shell_core_action_is_seen_by_the_monitor() {
    let bus = SharedBus::default();
    let monitor = Monitor::start("action");
    notify("live-mail", "act", "act-b=Run B");
    let info = listed(&bus).remove(0);
    let who = identity_of(&info);
    let request = NotificationActionRequest {
        index: info.index,
        identity: &who,
        action_name: "Run B",
        policy: InteractionPolicy::headed(),
    };
    let result = super::action(&bus, request, &lease()).expect("delivered");
    assert_ne!(
        result.disposition(),
        agent_desktop_core::DeliverySemantics::delivered_verified()
    );
    assert!(monitor.text().contains("ActionInvoked"));
}

#[test]
#[ignore = "needs the private gnome-shell and lane B's real shell core"]
fn needs_shell_core_dismiss_leaves_the_other_app_and_closes_with_reason_two() {
    let bus = SharedBus::default();
    let monitor = Monitor::start("dismiss");
    notify("live-keep", "stays", "");
    notify("live-drop", "goes", "");
    let info = listed(&bus).remove(0);
    let who = identity_of(&info);
    let request = DismissNotificationRequest {
        index: info.index,
        app_filter: None,
        identity: &who,
        policy: InteractionPolicy::headed(),
    };
    super::dismiss(&bus, request, &lease()).expect("dismissed");
    let left = listed(&bus);
    assert!(left.iter().any(|entry| entry.title == "stays"));
    assert!(!left.iter().any(|entry| entry.title == "goes"));
    assert!(monitor.text().contains("NotificationClosed"));
}

#[test]
#[ignore = "needs the private gnome-shell and lane B's real shell core"]
fn needs_shell_core_dismiss_all_empties_the_list() {
    let bus = SharedBus::default();
    notify("live-a", "one", "");
    notify("live-b", "two", "");
    let request = DismissAllNotificationsRequest {
        app_filter: None,
        policy: InteractionPolicy::headed(),
    };
    super::dismiss_all(&bus, request, &lease()).expect("cleared");
    assert!(listed(&bus).is_empty());
}

#[test]
#[ignore = "needs the private gnome-shell and lane B's real shell core"]
fn needs_shell_core_an_entry_that_has_gone_is_not_found() {
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
