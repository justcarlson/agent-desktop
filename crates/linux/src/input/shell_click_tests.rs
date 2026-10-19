use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use agent_desktop_core::{AdapterError, Deadline, ErrorCode, MouseButton, Point};

use crate::input::shell_click::{PointerSession, run_pointer};
use crate::input::wayland::key_session::KeySession;

#[derive(Default)]
struct Recording {
    log: Rc<RefCell<Vec<String>>>,
    fail_on: Option<&'static str>,
    position: (f64, f64),
}

impl Recording {
    fn failing_on(label: &'static str) -> Self {
        Self {
            fail_on: Some(label),
            ..Self::default()
        }
    }

    fn step(&mut self, label: String) -> Result<(), AdapterError> {
        self.log.borrow_mut().push(label.clone());
        if self.fail_on == Some(label.split(':').next().unwrap_or("")) {
            return Err(AdapterError::new(
                ErrorCode::ActionFailed,
                "the fake failed",
            ));
        }
        Ok(())
    }

    fn events(&self) -> Vec<String> {
        self.log.borrow().clone()
    }
}

impl KeySession for Recording {
    fn create(&mut self, _deadline: Deadline) -> Result<(), AdapterError> {
        self.step("create".to_owned())
    }

    fn start(&mut self, _deadline: Deadline) -> Result<(), AdapterError> {
        self.step("start".to_owned())
    }

    fn keysym(
        &mut self,
        _keysym: u32,
        _down: bool,
        _deadline: Deadline,
    ) -> Result<(), AdapterError> {
        self.step("keysym".to_owned())
    }

    fn stop(&mut self, _deadline: Deadline) -> Result<(), AdapterError> {
        self.step("stop".to_owned())
    }
}

impl PointerSession for Recording {
    fn motion(&mut self, dx: f64, dy: f64, _deadline: Deadline) -> Result<(), AdapterError> {
        if dx < -1_000.0 {
            self.position = (0.0, 0.0);
        } else {
            self.position = (self.position.0 + dx, self.position.1 + dy);
        }
        self.step("motion".to_owned())
    }

    fn button(&mut self, button: u32, down: bool, _deadline: Deadline) -> Result<(), AdapterError> {
        let label = format!(
            "button:{button}:{down}@{},{}",
            self.position.0, self.position.1
        );
        self.step(label)
    }

    fn pause(&mut self, _duration: Duration) {}
}

fn deadline() -> Deadline {
    Deadline::after(10_000).expect("a deadline builds")
}

fn target() -> Point {
    Point { x: 640.0, y: 16.0 }
}

#[test]
fn the_sequence_creates_starts_clamps_steps_clicks_and_stops() {
    let mut session = Recording::default();
    run_pointer(&mut session, &target(), &MouseButton::Left, deadline()).expect("the click lands");
    let events = session.events();
    assert_eq!(events.first().map(String::as_str), Some("create"));
    assert_eq!(events.get(1).map(String::as_str), Some("start"));
    assert_eq!(events.last().map(String::as_str), Some("stop"));
    assert_eq!(events[events.len() - 3], "button:272:true@640,16");
    assert_eq!(events[events.len() - 2], "button:272:false@640,16");
    let motions = events.iter().filter(|label| *label == "motion").count();
    assert!(motions >= 4 + 16, "four clamps and the steps to 640 pixels");
}

#[test]
fn the_right_button_is_the_right_code() {
    let mut session = Recording::default();
    run_pointer(&mut session, &target(), &MouseButton::Right, deadline()).expect("it clicks");
    assert!(
        session
            .events()
            .iter()
            .any(|label| label.starts_with("button:273:true"))
    );
}

#[test]
fn a_refused_creation_sends_no_event_and_needs_no_stop() {
    let mut session = Recording::failing_on("create");
    let error = run_pointer(&mut session, &target(), &MouseButton::Left, deadline())
        .expect_err("creation is refused");
    assert_eq!(error.code, ErrorCode::ActionFailed);
    assert_eq!(session.events(), ["create"]);
}

#[test]
fn a_failure_after_start_still_stops_the_session() {
    for label in ["motion", "start"] {
        let mut session = Recording::failing_on(label);
        run_pointer(&mut session, &target(), &MouseButton::Left, deadline())
            .expect_err("the sequence fails");
        assert_eq!(session.events().last().map(String::as_str), Some("stop"));
    }
}

#[test]
fn a_failed_button_press_still_stops_and_sends_no_release_after_it() {
    let mut session = Recording::failing_on("button");
    run_pointer(&mut session, &target(), &MouseButton::Left, deadline())
        .expect_err("the press fails");
    let events = session.events();
    assert_eq!(events.last().map(String::as_str), Some("stop"));
    assert_eq!(events.iter().filter(|l| l.starts_with("button")).count(), 1);
}

#[test]
fn a_session_that_will_not_stop_is_reported_after_a_good_click() {
    let mut session = Recording::failing_on("stop");
    let error = run_pointer(&mut session, &target(), &MouseButton::Left, deadline())
        .expect_err("the leak is an error");
    assert_eq!(error.code, ErrorCode::ActionFailed);
}
