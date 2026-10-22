use agent_desktop_core::ErrorCode;

use crate::system::shell::call_failure::CallFailure;
use crate::system::shell::detect::{
    Fact, Need, ShellEnvironment, classify_daemon, classify_tree, classify_watcher, require,
};
use crate::system::shell::watcher_read::WatcherRead;

fn environment(daemon: Fact, tree: Fact, watcher: Fact, server: Option<&str>) -> ShellEnvironment {
    ShellEnvironment {
        daemon,
        tree,
        watcher,
        server: server.map(str::to_owned),
    }
}

#[test]
fn daemon_requirement_uses_protocol_presence_instead_of_server_branding() {
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
    require(
        &environment(daemon, Fact::Present, Fact::Present, server.as_deref()),
        Need::Daemon,
    )
    .expect("a protocol reply is present regardless of its human-readable name");

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
    let absent = classify_watcher(Ok(WatcherRead::NoOwner));
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
            .contains("org.kde.StatusNotifierWatcher")
    );

    let present = classify_watcher(Ok(WatcherRead::Items));
    assert!(
        require(
            &environment(Fact::Present, Fact::Present, present, None),
            Need::Watcher,
        )
        .is_ok()
    );

    let vanished = classify_watcher(Err(CallFailure::NoService));
    let error = require(
        &environment(Fact::Present, Fact::Present, vanished, None),
        Need::Watcher,
    )
    .expect_err("watcher disappeared during read");
    assert_eq!(error.code, ErrorCode::Timeout);
    assert_eq!(error.details.expect("watcher details")["retryable"], true);

    let stalled = classify_watcher(Err(CallFailure::Transport("bus stalled".to_owned())));
    let error = require(
        &environment(Fact::Present, Fact::Present, stalled, None),
        Need::Watcher,
    )
    .expect_err("watcher transport failure");
    assert_eq!(error.code, ErrorCode::Timeout);
    assert_eq!(error.details.expect("watcher details")["retryable"], true);
}
