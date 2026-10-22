use agent_desktop_core::ErrorCode;

use super::{
    Fact, classify_daemon, classify_tree, classify_watcher, daemon_fact_error, watcher_fact_error,
};
use crate::system::shell::call_failure::CallFailure;
use crate::system::shell::watcher_read::WatcherRead;

#[test]
fn daemon_requirement_uses_protocol_presence_instead_of_server_branding() {
    for name in ["gnome-shell", "kded"] {
        let (daemon, server) = classify_daemon(Ok(name.to_owned()));
        daemon_fact_error(&daemon, server.as_deref()).expect("protocol presence ignores branding");
    }
    let (daemon, server) = classify_daemon(Err(CallFailure::NoService));
    let absent = daemon_fact_error(&daemon, server.as_deref()).expect_err("service absent");
    assert_eq!(absent.code, ErrorCode::PlatformNotSupported);
    assert_eq!(absent.details.unwrap()["daemon"], "absent");

    let (daemon, server) = classify_daemon(Err(CallFailure::Transport("bus stalled".into())));
    let unreadable = daemon_fact_error(&daemon, server.as_deref()).expect_err("failed read");
    assert_eq!(unreadable.code, ErrorCode::Timeout);
    assert_eq!(unreadable.details.unwrap()["retryable"], true);
}

#[test]
fn tree_detection_preserves_registry_failure_classes() {
    for (owner, registry) in [
        (Err(CallFailure::NoService), None),
        (Ok(7), Some(Ok(false))),
        (Ok(7), Some(Err(CallFailure::NoService))),
    ] {
        assert!(matches!(classify_tree(owner, registry), Fact::Absent(_)));
    }
    for (owner, registry) in [
        (Err(CallFailure::Transport("bus stalled".into())), None),
        (
            Ok(7),
            Some(Err(CallFailure::Transport("registry stalled".into()))),
        ),
    ] {
        assert!(matches!(
            classify_tree(owner, registry),
            Fact::Unreadable(_)
        ));
    }
    assert_eq!(classify_tree(Ok(7), Some(Ok(true))), Fact::Present);
}

#[test]
fn watcher_requirement_preserves_presence_and_races() {
    let absent = classify_watcher(Ok(WatcherRead::NoOwner));
    let error = watcher_fact_error(&absent).expect_err("watcher absent");
    assert_eq!(error.code, ErrorCode::PlatformNotSupported);
    assert!(
        error
            .suggestion
            .unwrap()
            .contains("org.kde.StatusNotifierWatcher")
    );
    watcher_fact_error(&classify_watcher(Ok(WatcherRead::Items))).expect("present watcher");

    for failure in [
        CallFailure::NoService,
        CallFailure::Transport("bus stalled".into()),
    ] {
        let error = watcher_fact_error(&classify_watcher(Err(failure)))
            .expect_err("a failed read cannot prove absence");
        assert_eq!(error.code, ErrorCode::Timeout);
        assert_eq!(error.details.unwrap()["retryable"], true);
    }
}
