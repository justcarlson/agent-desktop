use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use agent_desktop_core::ErrorCode;
use zbus::zvariant::{OwnedValue, Value};

use crate::system::shell::detect::{
    CallFailure, Fact, Need, SessionCalls, ShellEnvironment, TreeCalls, WatcherRead,
    classify_daemon, classify_tree, classify_watcher, detect_with, extension_state, require,
};

type Log = Rc<RefCell<Vec<&'static str>>>;

struct Session {
    log: Log,
    server: Result<String, CallFailure>,
    pid: Result<u32, CallFailure>,
    watcher: Result<WatcherRead, CallFailure>,
}

impl SessionCalls for Session {
    fn server_name(&self) -> Result<String, CallFailure> {
        self.log.borrow_mut().push("session:server");
        self.server.clone()
    }

    fn shell_pid(&self) -> Result<u32, CallFailure> {
        self.log.borrow_mut().push("session:pid");
        self.pid.clone()
    }

    fn watcher(&self) -> Result<WatcherRead, CallFailure> {
        self.log.borrow_mut().push("session:watcher");
        self.watcher.clone()
    }

    fn indicator_extension(&self) -> Option<String> {
        self.log.borrow_mut().push("session:extensions");
        Some("the AppIndicator extension is not installed".to_owned())
    }
}

struct Tree {
    log: Log,
    registered: Result<bool, CallFailure>,
}

impl TreeCalls for Tree {
    fn shell_registered(&self, _shell_pid: u32) -> Result<bool, CallFailure> {
        self.log.borrow_mut().push("a11y:registry");
        self.registered.clone()
    }
}

fn healthy(log: &Log) -> (Session, Tree) {
    (
        Session {
            log: Rc::clone(log),
            server: Ok("gnome-shell".to_owned()),
            pid: Ok(100),
            watcher: Ok(WatcherRead::Items(2)),
        },
        Tree {
            log: Rc::clone(log),
            registered: Ok(true),
        },
    )
}

#[test]
fn a_service_with_no_owner_is_a_daemon_that_is_absent() {
    let (fact, server) = classify_daemon(Err(CallFailure::NoService));
    assert!(matches!(fact, Fact::Absent(_)));
    assert_eq!(server, None);
}

#[test]
fn another_server_is_named_and_refused_as_unsupported() {
    let log = Log::default();
    let (mut session, tree) = healthy(&log);
    session.server = Ok("kded".to_owned());
    let env = detect_with(&session, &tree);
    assert_eq!(env.server.as_deref(), Some("kded"));
    let error = require(&env, Need::Daemon).expect_err("only GNOME is measured");
    assert_eq!(error.code, ErrorCode::PlatformNotSupported);
    assert!(error.message.contains("kded"));
}

#[test]
fn the_name_field_decides_so_a_helper_owned_daemon_is_present() {
    let log = Log::default();
    let (session, tree) = healthy(&log);
    let env = detect_with(&session, &tree);
    assert_eq!(env.daemon, Fact::Present);
    assert_eq!(env.tree, Fact::Present);
    assert!(require(&env, Need::Daemon).is_ok());
}

#[test]
fn a_shell_missing_from_the_registry_is_tree_absent_with_the_restart_suggestion() {
    let log = Log::default();
    let (session, mut tree) = healthy(&log);
    tree.registered = Ok(false);
    let env = detect_with(&session, &tree);
    assert!(matches!(env.tree, Fact::Absent(_)));
    assert_eq!(env.daemon, Fact::Present);
    let error = require(&env, Need::Tree).expect_err("the tree is absent");
    assert_eq!(error.code, ErrorCode::AppNotFound);
    let suggestion = error.suggestion.expect("a suggestion");
    assert!(suggestion.contains("Restart the shell session"));
    assert!(suggestion.contains("do not toggle accessibility"));
}

#[test]
fn a_shell_with_no_bus_owner_is_tree_absent_without_reading_the_registry() {
    let log = Log::default();
    let (mut session, tree) = healthy(&log);
    session.pid = Err(CallFailure::NoService);
    let env = detect_with(&session, &tree);
    assert!(matches!(env.tree, Fact::Absent(_)));
    assert!(!log.borrow().contains(&"a11y:registry"));
}

#[test]
fn a_watcher_with_no_owner_is_absent_and_carries_the_extension_state() {
    let log = Log::default();
    let (mut session, tree) = healthy(&log);
    session.watcher = Ok(WatcherRead::NoOwner);
    let env = detect_with(&session, &tree);
    let Fact::Absent(why) = &env.watcher else {
        panic!("a watcher with no owner is absent");
    };
    assert!(why.contains("AppIndicator extension is not installed"));
    let error = require(&env, Need::Watcher).expect_err("no watcher");
    assert!(
        error
            .suggestion
            .expect("a suggestion")
            .contains("AppIndicator")
    );
}

#[test]
fn a_watcher_with_no_items_is_present_and_empty() {
    assert_eq!(
        classify_watcher(Ok(WatcherRead::Items(0)), None),
        Fact::Present
    );
}

#[test]
fn a_watcher_name_that_vanishes_mid_read_is_retryable_not_absent() {
    let fact = classify_watcher(Err(CallFailure::NoService), None);
    assert!(matches!(fact, Fact::Unreadable(_)));
    let env = ShellEnvironment {
        daemon: Fact::Present,
        tree: Fact::Present,
        watcher: fact,
        server: None,
    };
    let error = require(&env, Need::Watcher).expect_err("unreadable");
    let details = error.details.expect("details");
    assert_eq!(details["retryable"], true);
}

#[test]
fn a_transport_failure_is_unreadable_for_every_fact() {
    let failure = || CallFailure::Transport("boom".to_owned());
    assert!(matches!(
        classify_daemon(Err(failure())).0,
        Fact::Unreadable(_)
    ));
    assert!(matches!(
        classify_tree(Ok(1), Some(Err(failure()))),
        Fact::Unreadable(_)
    ));
    assert!(matches!(
        classify_watcher(Err(failure()), None),
        Fact::Unreadable(_)
    ));
}

#[test]
fn every_session_call_goes_to_the_session_and_every_registry_read_to_the_accessibility_bus() {
    let log = Log::default();
    let (session, tree) = healthy(&log);
    detect_with(&session, &tree);
    assert_eq!(
        *log.borrow(),
        [
            "session:server",
            "session:pid",
            "a11y:registry",
            "session:watcher"
        ]
    );
}

#[test]
fn the_three_states_are_independent() {
    let log = Log::default();
    let (mut session, mut tree) = healthy(&log);
    session.server = Err(CallFailure::NoService);
    tree.registered = Ok(true);
    session.watcher = Ok(WatcherRead::NoOwner);
    let env = detect_with(&session, &tree);
    assert!(matches!(env.daemon, Fact::Absent(_)));
    assert_eq!(env.tree, Fact::Present);
    assert!(matches!(env.watcher, Fact::Absent(_)));
}

fn listed(uuid: &str, state: f64) -> HashMap<String, HashMap<String, OwnedValue>> {
    let value = OwnedValue::try_from(Value::F64(state)).expect("a value");
    HashMap::from([(
        uuid.to_owned(),
        HashMap::from([("state".to_owned(), value)]),
    )])
}

#[test]
fn the_extension_state_names_missing_disabled_and_enabled() {
    assert!(extension_state(&HashMap::new()).contains("not installed"));
    let disabled = extension_state(&listed("appindicator-support@fixture", 2.0));
    assert!(disabled.contains("not enabled"));
    let enabled = extension_state(&listed("appindicator-support@fixture", 1.0));
    assert!(enabled.contains("installed and enabled"));
}
