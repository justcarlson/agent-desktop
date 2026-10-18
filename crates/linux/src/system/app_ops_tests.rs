use agent_desktop_core::{AppInfo, ErrorCode};

use super::scoped_outcome;

fn app() -> AppInfo {
    AppInfo {
        name: "gnome-calculator".to_owned(),
        pid: 5.into(),
        bundle_id: None,
        process_instance: Some("b:1".to_owned()),
        presentation: None,
    }
}

#[test]
fn a_registry_match_is_returned_without_scanning_proc() {
    let found = scoped_outcome(vec![app()], "gnome-calculator", |_| {
        panic!("proc must not be scanned")
    })
    .expect("matched");
    assert_eq!(found.len(), 1);
}

#[test]
fn a_running_process_off_the_bus_is_app_not_found_naming_the_connection() {
    let error = scoped_outcome(Vec::new(), "firefox", |_| Some((77, "firefox".to_owned())))
        .expect_err("diagnostic");
    assert_eq!(error.code, ErrorCode::AppNotFound);
    assert!(error.message.contains("not on the accessibility bus"));
    assert!(error.message.contains("77"));
    let details = error.details.expect("details");
    assert_eq!(details["pid"], 77);
}

#[test]
fn a_name_nothing_matches_is_the_ordinary_empty_answer() {
    let found = scoped_outcome(Vec::new(), "nothing", |_| None).expect("empty");
    assert!(found.is_empty());
}
