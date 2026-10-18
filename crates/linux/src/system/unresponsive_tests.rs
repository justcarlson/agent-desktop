use super::{require_known_target, stopped_target};
use crate::system::process_facts::ProcessFacts;
use crate::system::registry::Registry;
use crate::system::registry_app::{RegistryApp, assemble};
use crate::system::toolkit_id::ToolkitId;
use crate::tree::AtspiElement;
use agent_desktop_core::ErrorCode;

fn app(pid: u32, exe: &str, answered: bool) -> RegistryApp {
    assemble(
        AtspiElement::new(":1.9", "/org/a11y/atspi/accessible/root", pid),
        ProcessFacts {
            comm: exe.to_owned(),
            exe_basename: Some(exe.to_owned()),
            instance: "boot:1".to_owned(),
            web_family: None,
        },
        answered.then(|| Some("Name".to_owned())),
        ToolkitId::default(),
    )
}

fn registry(apps: Vec<RegistryApp>) -> Registry {
    Registry {
        apps,
        skipped: Vec::new(),
    }
}

#[test]
fn an_unanswering_stopped_target_is_app_unresponsive_naming_the_app() {
    let registry = registry(vec![app(7, "nautilus", false)]);
    let error = stopped_target(&registry, |_| true, |pid| pid == 7).expect("stopped");
    assert_eq!(error.code, ErrorCode::AppUnresponsive);
    assert!(error.message.contains("nautilus"));
}

#[test]
fn an_unanswering_app_that_is_not_stopped_is_not_reported() {
    let registry = registry(vec![app(7, "nautilus", false)]);
    assert!(stopped_target(&registry, |_| true, |_| false).is_none());
}

#[test]
fn a_stopped_looking_pid_of_an_answering_app_is_not_reported() {
    let registry = registry(vec![app(7, "gnome-calculator", true)]);
    assert!(stopped_target(&registry, |_| true, |_| true).is_none());
}

#[test]
fn only_the_selected_apps_are_considered() {
    let registry = registry(vec![
        app(7, "nautilus", false),
        app(8, "gnome-calculator", true),
    ]);
    assert!(stopped_target(&registry, |app| app.root.pid() == 8, |_| true).is_none());
    assert!(stopped_target(&registry, |app| app.root.pid() == 7, |_| true).is_some());
}

#[test]
fn a_helper_app_is_never_the_target() {
    let registry = registry(vec![app(7, "gnome-shell", false)]);
    assert!(stopped_target(&registry, |_| true, |_| true).is_none());
}

#[test]
fn an_unknown_name_is_app_not_found_suggesting_list_apps() {
    let registry = registry(vec![app(7, "nautilus", true)]);
    let error =
        require_known_target(&registry, |_| false, "Calculator", |_| None).expect_err("unknown");
    assert_eq!(error.code, ErrorCode::AppNotFound);
    assert!(error.message.contains("Calculator"));
    assert!(error.suggestion.expect("suggestion").contains("list-apps"));
}

#[test]
fn a_registered_app_without_windows_is_not_an_unknown_name() {
    let registry = registry(vec![app(7, "nautilus", true)]);
    let scanned = |_: &str| panic!("proc must not be scanned");
    assert!(require_known_target(&registry, |_| true, "nautilus", scanned).is_ok());
}

#[test]
fn a_bus_less_process_keeps_the_empty_list() {
    let registry = registry(Vec::new());
    let running = |_: &str| Some((77, "sleep".to_owned()));
    assert!(require_known_target(&registry, |_| true, "sleep", running).is_ok());
}
