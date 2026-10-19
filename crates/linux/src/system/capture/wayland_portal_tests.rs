use std::cell::{Cell, RefCell};
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

use agent_desktop_core::{DisplayInfo, Rect};

use super::*;
use crate::system::capture::wayland_portal_call::{Pending, PortalAnswer, request_path};

struct Answer(Result<PortalAnswer, AdapterError>);

impl Pending for Answer {
    fn wait(self: Box<Self>, _deadline: Deadline) -> Result<PortalAnswer, AdapterError> {
        self.0
    }
}

struct Wire {
    answer: Result<PortalAnswer, AdapterError>,
    calls: Rc<RefCell<Vec<String>>>,
}

impl PortalWire for Wire {
    fn unique_name(&self) -> String {
        ":1.9".into()
    }

    fn subscribe(&self, path: &str, _deadline: Deadline) -> Result<Box<dyn Pending>, AdapterError> {
        self.calls.borrow_mut().push(format!("subscribe {path}"));
        Ok(Box::new(Answer(self.answer.clone())))
    }

    fn call_screenshot(&self, token: &str, _deadline: Deadline) -> Result<String, AdapterError> {
        self.calls.borrow_mut().push("call".into());
        Ok(request_path(":1.9", token))
    }
}

fn deadline() -> Deadline {
    Deadline::after(5_000).expect("a deadline")
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adwp-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("a directory");
    dir
}

fn tiny_png(width: u32, height: u32) -> Vec<u8> {
    let mut data = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 13];
    data.extend_from_slice(b"IHDR");
    data.extend_from_slice(&width.to_be_bytes());
    data.extend_from_slice(&height.to_be_bytes());
    data.extend_from_slice(&[8, 2, 0, 0, 0]);
    data.extend_from_slice(&[0; 4]);
    data
}

fn display() -> DisplayInfo {
    DisplayInfo {
        id: "0".into(),
        bounds: Rect {
            x: 0.0,
            y: 0.0,
            width: 1280.0,
            height: 800.0,
        },
        scale: 1.0,
        is_primary: true,
    }
}

fn run(
    target: &ScreenshotTarget,
    answer: Result<PortalAnswer, AdapterError>,
    roots: &[PathBuf],
) -> (Result<ImageBuffer, AdapterError>, Vec<String>) {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let connect = {
        let calls = Rc::clone(&calls);
        move |_deadline: Deadline| -> Result<Box<dyn PortalWire>, AdapterError> {
            Ok(Box::new(Wire {
                answer: answer.clone(),
                calls: Rc::clone(&calls),
            }))
        }
    };
    let outcome = capture_with(target, deadline(), &connect, roots);
    let seen = calls.borrow().clone();
    (outcome, seen)
}

fn granted(path: &std::path::Path) -> Result<PortalAnswer, AdapterError> {
    Ok(PortalAnswer::Granted {
        uri: Some(format!("file://{}", path.display())),
    })
}

#[test]
fn the_whole_screen_is_the_portals_png_and_its_file_is_removed() {
    let dir = scratch("whole");
    let path = dir.join("Screenshot.png");
    fs::write(&path, tiny_png(1280, 800)).expect("a png");
    let (outcome, calls) = run(&ScreenshotTarget::FullScreen, granted(&path), &[dir]);
    let image = outcome.expect("an image");
    assert_eq!((image.width, image.height), (1280, 800));
    assert_eq!(calls.len(), 2);
    assert!(!path.exists());
}

#[test]
fn a_file_that_is_not_a_png_is_an_action_failure_and_is_still_removed() {
    let dir = scratch("junk");
    let path = dir.join("Screenshot.png");
    fs::write(&path, b"not a png at all, only text").expect("a file");
    let (outcome, _) = run(&ScreenshotTarget::FullScreen, granted(&path), &[dir]);
    assert_eq!(outcome.expect_err("junk").code, ErrorCode::ActionFailed);
    assert!(!path.exists());
}

#[test]
fn a_uri_outside_the_allowed_directories_is_refused_and_its_file_kept() {
    let dir = scratch("inside");
    let elsewhere = scratch("outside-uri");
    let path = elsewhere.join("Screenshot.png");
    fs::write(&path, tiny_png(4, 4)).expect("a png");
    let (outcome, _) = run(&ScreenshotTarget::FullScreen, granted(&path), &[dir]);
    assert_eq!(outcome.expect_err("refused").code, ErrorCode::ActionFailed);
    assert!(path.exists());
}

#[test]
fn a_refusal_from_the_portal_is_permission_denied() {
    let (outcome, _) = run(
        &ScreenshotTarget::FullScreen,
        Ok(PortalAnswer::Refused { code: 2 }),
        &[],
    );
    assert_eq!(outcome.expect_err("denied").code, ErrorCode::PermDenied);
}

#[test]
fn a_display_target_is_unsupported_and_makes_no_portal_call() {
    for target in [
        ScreenshotTarget::Screen(0),
        ScreenshotTarget::Display {
            index: 0,
            expected: display(),
        },
    ] {
        let (outcome, calls) = run(&target, Ok(PortalAnswer::Refused { code: 2 }), &[]);
        let error = outcome.expect_err("unsupported");
        assert_eq!(error.code, ErrorCode::PlatformNotSupported);
        assert!(calls.is_empty());
    }
}

#[test]
fn a_missing_session_bus_is_unsupported_and_does_not_hang() {
    let dialed = Cell::new(false);
    let connect = |_deadline: Deadline| -> Result<Box<dyn PortalWire>, AdapterError> {
        dialed.set(true);
        Err(AdapterError::new(
            ErrorCode::PlatformNotSupported,
            "The D-Bus session bus is not available in this session",
        ))
    };
    let outcome = capture_with(&ScreenshotTarget::FullScreen, deadline(), &connect, &[]);
    assert_eq!(
        outcome.expect_err("no bus").code,
        ErrorCode::PlatformNotSupported
    );
    assert!(dialed.get());
}
