use agent_desktop_core::{ErrorCode, ProcessId, WindowState};

use super::*;

fn missing() -> AdapterError {
    AdapterError::new(
        ErrorCode::AppNotFound,
        "No running application matches 'ad-term'",
    )
}

fn terminal() -> WindowInfo {
    WindowInfo {
        id: "w-hypr-666-345-18000006".into(),
        title: "shell".into(),
        app: "ad-term".into(),
        pid: ProcessId::new(3109),
        process_instance: Some("boot:3109".into()),
        bounds: None,
        state: WindowState {
            is_focused: false,
            accessible: false,
            minimized: None,
            visible: Some(true),
        },
    }
}

#[test]
fn a_name_only_a_compositor_toplevel_answers_to_lists_that_toplevel() {
    let windows = unregistered_target(missing(), || Ok(vec![terminal()]));
    assert_eq!(windows.map(|found| found.len()).ok(), Some(1));
}

#[test]
fn a_name_nothing_answers_to_stays_app_not_found() {
    let error = unregistered_target(missing(), || Ok(Vec::new())).err();
    assert_eq!(error.map(|error| error.code), Some(ErrorCode::AppNotFound));
}

#[test]
fn a_compositor_that_cannot_be_read_reports_its_own_error() {
    let error = unregistered_target(missing(), || Err(AdapterError::timeout("silent"))).err();
    assert_eq!(error.map(|error| error.code), Some(ErrorCode::Timeout));
}
