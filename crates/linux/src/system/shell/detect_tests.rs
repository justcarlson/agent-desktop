use std::collections::HashMap;

use agent_desktop_core::ErrorCode;
use zbus::zvariant::{OwnedValue, Value};

use crate::system::shell::detect::{
    CallFailure, Fact, Need, ShellEnvironment, WatcherRead, classify_daemon, classify_tree,
    classify_watcher, extension_state, require,
};

fn environment(daemon: Fact, tree: Fact, watcher: Fact, server: Option<&str>) -> ShellEnvironment {
    ShellEnvironment {
        daemon,
        tree,
        watcher,
        server: server.map(str::to_owned),
    }
}

#[test]
fn daemon_requirement_refuses_absent_and_foreign_servers() {
    let (daemon, server) = classify_daemon(Ok("gnome-shell".to_owned()));
    assert!(
        require(
            &environment(daemon, Fact::Present, Fact::Present, server.as_deref()),
            Need::Daemon,
        )
        .is_ok()
    );

    let (daemon, server) = classify_daemon(Err(CallFailure::NoService));
    let absent = require(
        &environment(daemon, Fact::Present, Fact::Present, server.as_deref()),
        Need::Daemon,
    )
    .expect_err("no notification service");
    assert_eq!(absent.code, ErrorCode::PlatformNotSupported);
    assert_eq!(absent.details.expect("daemon details")["daemon"], "absent");

    let (daemon, server) = classify_daemon(Ok("kded".to_owned()));
    let foreign = require(
        &environment(daemon, Fact::Present, Fact::Present, server.as_deref()),
        Need::Daemon,
    )
    .expect_err("foreign notification service");
    assert_eq!(foreign.code, ErrorCode::PlatformNotSupported);
    assert!(foreign.message.contains("kded"));

    let (daemon, server) = classify_daemon(Err(CallFailure::Transport("bus stalled".to_owned())));
    let unreadable = require(
        &environment(daemon, Fact::Present, Fact::Present, server.as_deref()),
        Need::Daemon,
    )
    .expect_err("daemon transport failure");
    assert_eq!(unreadable.code, ErrorCode::Timeout);
    assert_eq!(
        unreadable.details.expect("daemon details")["retryable"],
        true
    );
}

#[test]
fn tree_requirement_preserves_registry_failure_classes() {
    let absent_owner = classify_tree(Err(CallFailure::NoService), None);
    let error = require(
        &environment(Fact::Present, absent_owner, Fact::Present, None),
        Need::Tree,
    )
    .expect_err("shell owner absent");
    assert_eq!(error.code, ErrorCode::AppNotFound);
    assert_eq!(
        error.details.expect("absence evidence")["shell_tree"],
        "absent"
    );

    let owner_unreadable =
        classify_tree(Err(CallFailure::Transport("bus stalled".to_owned())), None);
    let error = require(
        &environment(Fact::Present, owner_unreadable, Fact::Present, None),
        Need::Tree,
    )
    .expect_err("shell owner transport failure");
    assert_eq!(error.code, ErrorCode::Timeout);
    assert_eq!(error.details.expect("tree details")["retryable"], true);

    let absent_registry = classify_tree(Ok(7), Some(Ok(false)));
    let error = require(
        &environment(Fact::Present, absent_registry, Fact::Present, None),
        Need::Tree,
    )
    .expect_err("shell missing from registry");
    assert_eq!(error.code, ErrorCode::AppNotFound);
    let registry_unavailable = classify_tree(Ok(7), Some(Err(CallFailure::NoService)));
    assert!(matches!(registry_unavailable, Fact::Absent(_)));
    let unreadable = classify_tree(
        Ok(7),
        Some(Err(CallFailure::Transport("registry stalled".to_owned()))),
    );
    let error = require(
        &environment(Fact::Present, unreadable, Fact::Present, None),
        Need::Tree,
    )
    .expect_err("registry transport failure");
    assert_eq!(error.code, ErrorCode::Timeout);
    assert_eq!(
        error.details.expect("tree details")["shell_tree"],
        "unreadable"
    );
}

#[test]
fn watcher_requirement_preserves_presence_and_races() {
    let absent = classify_watcher(Ok(WatcherRead::NoOwner), Some("not installed".to_owned()));
    let error = require(
        &environment(Fact::Present, Fact::Present, absent, None),
        Need::Watcher,
    )
    .expect_err("watcher absent");
    assert_eq!(error.code, ErrorCode::PlatformNotSupported);
    assert!(
        error
            .suggestion
            .expect("extension guidance")
            .contains("AppIndicator")
    );

    let present = classify_watcher(Ok(WatcherRead::Items), None);
    assert!(
        require(
            &environment(Fact::Present, Fact::Present, present, None),
            Need::Watcher,
        )
        .is_ok()
    );

    let vanished = classify_watcher(Err(CallFailure::NoService), None);
    let error = require(
        &environment(Fact::Present, Fact::Present, vanished, None),
        Need::Watcher,
    )
    .expect_err("watcher disappeared during read");
    assert_eq!(error.code, ErrorCode::Timeout);
    assert_eq!(error.details.expect("watcher details")["retryable"], true);

    let stalled = classify_watcher(Err(CallFailure::Transport("bus stalled".to_owned())), None);
    let error = require(
        &environment(Fact::Present, Fact::Present, stalled, None),
        Need::Watcher,
    )
    .expect_err("watcher transport failure");
    assert_eq!(error.code, ErrorCode::Timeout);
    assert_eq!(error.details.expect("watcher details")["retryable"], true);
}

fn listed(uuid: &str, state: f64) -> HashMap<String, HashMap<String, OwnedValue>> {
    let value = OwnedValue::try_from(Value::F64(state)).expect("valid extension state");
    HashMap::from([(
        uuid.to_owned(),
        HashMap::from([("state".to_owned(), value)]),
    )])
}

#[test]
fn extension_diagnostics_preserve_three_states() {
    assert!(extension_state(&HashMap::new()).contains("not installed"));
    assert!(extension_state(&listed("appindicator-support@fixture", 2.0)).contains("not enabled"));
    assert!(
        extension_state(&listed("appindicator-support@fixture", 1.0))
            .contains("installed and enabled")
    );
}
