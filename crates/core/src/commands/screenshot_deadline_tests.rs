use super::*;
use crate::adapter::{ActionOps, InputOps, ObservationOps, SystemOps};
use std::{sync::Mutex, time::Duration};

struct ProbeBudgetAdapter {
    probe_budget: Mutex<Option<Duration>>,
}

impl ObservationOps for ProbeBudgetAdapter {}
impl ActionOps for ProbeBudgetAdapter {}
impl InputOps for ProbeBudgetAdapter {}

impl SystemOps for ProbeBudgetAdapter {
    fn process_state(
        &self,
        _process: crate::ProcessIdentity,
        deadline: Deadline,
    ) -> Result<ProcessState, AdapterError> {
        *self.probe_budget.lock().unwrap() = Some(deadline.remaining());
        Ok(ProcessState::Running)
    }
}

#[test]
fn stopped_app_probe_never_outlives_the_command_deadline() {
    let adapter = ProbeBudgetAdapter {
        probe_budget: Mutex::new(None),
    };
    let app = crate::AppInfo {
        name: "Example".into(),
        pid: crate::ProcessId::new(42),
        bundle_id: None,
        process_instance: Some("test-instance".into()),
        presentation: None,
    };
    let deadline = Deadline::detached_after(200).unwrap();

    let _ = missing_window_error("w-1", Some(&app), &adapter, deadline);

    let budget = adapter.probe_budget.lock().unwrap().expect("probe ran");
    assert!(budget <= Duration::from_millis(200), "{budget:?}");
}

struct SilentAdapter;

impl ObservationOps for SilentAdapter {
    fn list_windows(
        &self,
        _filter: &WindowFilter,
        _deadline: Deadline,
    ) -> Result<Vec<crate::WindowInfo>, AdapterError> {
        Err(AdapterError::timeout(
            "The compositor did not answer in time",
        ))
    }
}
impl ActionOps for SilentAdapter {}
impl InputOps for SilentAdapter {}

impl SystemOps for SilentAdapter {
    fn list_displays(
        &self,
        _deadline: Deadline,
    ) -> Result<Vec<crate::system::display_info::DisplayInfo>, AdapterError> {
        Ok(vec![crate::system::display_info::DisplayInfo {
            id: "main".into(),
            bounds: crate::Rect {
                x: 0.0,
                y: 0.0,
                width: 800.0,
                height: 600.0,
            },
            is_primary: true,
            scale: 1.0,
        }])
    }

    fn screenshot(
        &self,
        _target: ScreenshotTarget,
        _deadline: Deadline,
    ) -> Result<crate::ImageBuffer, AdapterError> {
        Err(AdapterError::timeout("The frame did not arrive in time"))
    }
}

#[test]
fn a_capture_that_fails_before_the_write_saved_nothing_so_its_retry_is_safe() {
    let folder = std::env::temp_dir().join(format!("ad-screenshot-unsent-{}", std::process::id()));
    let path = folder.join("never.png");
    let by_window = ScreenshotArgs {
        app: None,
        window_id: Some("w-1".into()),
        screen: None,
        output_path: Some(path.clone()),
    };
    let by_screen = ScreenshotArgs {
        app: None,
        window_id: None,
        screen: Some(0),
        output_path: Some(path.clone()),
    };

    for request in [by_window, by_screen] {
        let Err(AppError::Adapter(error)) = execute(request, &SilentAdapter) else {
            panic!("a timeout");
        };
        assert_eq!(error.code, ErrorCode::Timeout);
        assert_eq!(error.disposition, crate::DeliverySemantics::not_delivered());
    }
    assert!(!path.exists());
}
