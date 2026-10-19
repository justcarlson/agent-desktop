use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use agent_desktop_core::{
    AdapterError, Deadline, DisplayInfo, ErrorCode, MouseButton, Point, Rect,
};

use crate::input::shell_click::{
    Aim, PointerSession, in_modal, monitor_under, run_pointer, run_pointer_absolute,
};
use crate::input::wayland::key_session::KeySession;

#[derive(Default)]
struct Recording {
    log: Rc<RefCell<Vec<String>>>,
    fail_on: Option<&'static str>,
    position: (f64, f64),
    recovery_live: Vec<(&'static str, bool)>,
    stream: bool,
    start_fails: bool,
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
        if self.start_fails {
            self.log.borrow_mut().push("start".to_owned());
            return Err(AdapterError::new(ErrorCode::ActionFailed, "no pipewire"));
        }
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

    fn stop(&mut self, deadline: Deadline) -> Result<(), AdapterError> {
        self.recovery_live.push(("stop", !deadline.is_expired()));
        self.step("stop".to_owned())
    }
}

impl PointerSession for Recording {
    fn motion(&mut self, dx: f64, dy: f64, _deadline: Deadline) -> Result<(), AdapterError> {
        if dx <= -500.0 {
            self.position = (0.0, 0.0);
        } else {
            self.position = (self.position.0 + dx, self.position.1 + dy);
        }
        self.step("motion".to_owned())
    }

    fn button(&mut self, button: u32, down: bool, deadline: Deadline) -> Result<(), AdapterError> {
        if !down {
            self.recovery_live.push(("up", !deadline.is_expired()));
        }
        let label = format!(
            "button:{button}:{down}@{},{}",
            self.position.0, self.position.1
        );
        self.step(label)
    }

    fn pause(&mut self, _duration: Duration) {}

    fn attach_stream(&mut self, connector: &str, _deadline: Deadline) -> bool {
        self.log.borrow_mut().push(format!("attach:{connector}"));
        self.stream
    }

    fn motion_absolute(&mut self, x: f64, y: f64, _deadline: Deadline) -> Result<(), AdapterError> {
        self.position = (x, y);
        self.step(format!("absolute@{x},{y}"))
    }
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

#[test]
fn the_release_and_the_stop_run_on_a_live_deadline_after_the_budget_is_spent() {
    let spent = Deadline::after(1).expect("a deadline builds");
    std::thread::sleep(Duration::from_millis(20));
    let mut session = Recording::default();
    run_pointer(&mut session, &target(), &MouseButton::Left, spent).expect("the click lands");
    assert_eq!(session.recovery_live, [("up", true), ("stop", true)]);
}

fn aim(modal: bool) -> Aim {
    Aim {
        target: Point { x: 677.0, y: 71.0 },
        connector: "Meta-0".to_owned(),
        local: Point { x: 677.0, y: 71.0 },
        in_modal: modal,
    }
}

fn session_with_stream() -> Recording {
    Recording {
        stream: true,
        ..Recording::default()
    }
}

#[test]
fn a_linked_stream_aims_the_click_with_one_absolute_motion_and_no_relative_travel() {
    let mut session = session_with_stream();
    run_pointer_absolute(&mut session, &aim(true), &MouseButton::Left, deadline())
        .expect("the click lands");
    let events = session.events();
    assert_eq!(events[0], "create");
    assert_eq!(events[1], "attach:Meta-0");
    assert_eq!(events[2], "start");
    assert_eq!(events[3], "absolute@677,71");
    assert_eq!(events[4], "button:272:true@677,71");
    assert_eq!(events[5], "button:272:false@677,71");
    assert_eq!(events.last().map(String::as_str), Some("stop"));
    assert!(!events.iter().any(|label| label == "motion"));
}

#[test]
fn a_click_on_the_top_bar_without_a_stream_goes_on_with_relative_motion() {
    let mut session = Recording::default();
    run_pointer_absolute(&mut session, &aim(false), &MouseButton::Left, deadline())
        .expect("the click lands");
    let events = session.events();
    assert!(events.iter().any(|label| label == "motion"));
    assert!(
        events
            .iter()
            .any(|label| label.starts_with("button:272:true"))
    );
    assert!(!events.iter().any(|label| label.starts_with("absolute")));
}

#[test]
fn a_click_inside_a_popup_without_a_stream_is_refused_before_any_event() {
    let mut session = Recording::default();
    let error = run_pointer_absolute(&mut session, &aim(true), &MouseButton::Left, deadline())
        .expect_err("refused");
    assert_eq!(error.code, ErrorCode::ActionFailed);
    let details = error.details.expect("details");
    assert_eq!(details["cause"], "no_screencast_stream");
    assert_eq!(details["physical_delivery_started"], false);
    let events = session.events();
    assert!(
        !events
            .iter()
            .any(|label| label == "motion" || label.starts_with("button"))
    );
}

#[test]
fn a_stream_that_will_not_start_in_a_popup_is_refused_with_nothing_sent() {
    let mut session = Recording {
        stream: true,
        start_fails: true,
        ..Recording::default()
    };
    let error = run_pointer_absolute(&mut session, &aim(true), &MouseButton::Left, deadline())
        .expect_err("refused");
    assert_eq!(
        error.details.expect("details")["cause"],
        "no_screencast_stream"
    );
    assert!(
        !session
            .events()
            .iter()
            .any(|label| label.starts_with("button"))
    );
}

fn display(id: &str, primary: bool, x: f64, y: f64) -> DisplayInfo {
    DisplayInfo {
        id: id.to_owned(),
        bounds: Rect {
            x,
            y,
            width: 1280.0,
            height: 800.0,
        },
        is_primary: primary,
        scale: 1.0,
    }
}

#[test]
fn the_monitor_that_holds_the_point_is_chosen_and_the_point_made_local_to_it() {
    let displays = [
        display("DP-1", true, 100.0, 50.0),
        display("HDMI-1+DP-2", false, 1380.0, 50.0),
    ];
    let (first, local) = monitor_under(&Point { x: 140.0, y: 70.0 }, &displays).expect("first");
    assert_eq!((first.as_str(), local.x, local.y), ("DP-1", 40.0, 20.0));
    let (second, local) = monitor_under(&Point { x: 1500.0, y: 70.0 }, &displays).expect("second");
    assert_eq!((second.as_str(), local.x, local.y), ("HDMI-1", 120.0, 20.0));
    assert!(monitor_under(&Point { x: 1.0, y: 1.0 }, &displays).is_none());
}

#[test]
fn only_a_point_below_the_top_bar_is_inside_a_modal() {
    assert!(!in_modal(&Point { x: 640.0, y: 16.0 }));
    assert!(in_modal(&Point { x: 677.0, y: 71.0 }));
}
