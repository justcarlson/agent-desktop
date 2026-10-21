use std::time::Duration;

use agent_desktop_core::{
    AdapterError, Deadline, DeliverySemantics, DisplayInfo, ErrorCode, MouseButton, Point, Rect,
};

use crate::input::{
    shell_click::{
        Aim, PointerSession, in_modal, monitor_under, run_pointer, run_pointer_absolute,
    },
    wayland::key_session::KeySession,
};

#[derive(Default)]
struct Recording {
    log: Vec<String>,
    fail: Option<&'static str>,
    pos: (f64, f64),
    recovery: Vec<(&'static str, bool)>,
    stream: bool,
    start_fails: bool,
}
impl Recording {
    fn fail(fail: &'static str) -> Self {
        Self {
            fail: Some(fail),
            ..Self::default()
        }
    }
    fn step(&mut self, event: &str) -> Result<(), AdapterError> {
        self.log.push(event.into());
        if self.fail == Some(event) {
            return Err(if event == "up" || event == "stop" {
                crate::input::wayland::remote_desktop::semantic_action_refusal("refused")
            } else {
                AdapterError::new(ErrorCode::ActionFailed, "failed")
            });
        }
        Ok(())
    }
}
impl KeySession for Recording {
    fn create(&mut self, _: Deadline) -> Result<(), AdapterError> {
        self.step("create")
    }
    fn start(&mut self, _: Deadline) -> Result<(), AdapterError> {
        if self.start_fails {
            self.log.push("start".into());
            Err(AdapterError::new(ErrorCode::ActionFailed, "no stream"))
        } else {
            self.step("start")
        }
    }
    fn keysym(&mut self, _: u32, _: bool, _: Deadline) -> Result<(), AdapterError> {
        Ok(())
    }
    fn stop(&mut self, d: Deadline) -> Result<(), AdapterError> {
        self.recovery.push(("stop", !d.is_expired()));
        self.step("stop")
    }
}
impl PointerSession for Recording {
    fn motion(&mut self, dx: f64, dy: f64, _: Deadline) -> Result<(), AdapterError> {
        if dx <= -500. {
            self.pos = (0., 0.)
        } else {
            self.pos = (self.pos.0 + dx, self.pos.1 + dy)
        }
        self.step("motion")
    }
    fn button(&mut self, _: u32, down: bool, d: Deadline) -> Result<(), AdapterError> {
        if !down {
            self.recovery.push(("up", !d.is_expired()))
        }
        self.step(if down { "down" } else { "up" })
    }
    fn pause(&mut self, _: Duration) {}
    fn attach_stream(&mut self, _: &str, _: Deadline) -> bool {
        self.log.push("attach".into());
        self.stream
    }
    fn motion_absolute(&mut self, x: f64, y: f64, _: Deadline) -> Result<(), AdapterError> {
        self.pos = (x, y);
        self.step("absolute")
    }
}
fn deadline() -> Deadline {
    Deadline::after(10_000).expect("deadline")
}
fn target() -> Point {
    Point { x: 640., y: 16. }
}
fn aim(modal: bool) -> Aim {
    Aim {
        target: Point { x: 677., y: 71. },
        connector: "Meta-0".into(),
        local: Point { x: 77., y: 21. },
        in_modal: modal,
    }
}
fn check(absolute: bool, stage: &'static str) {
    let mut s = if absolute {
        Recording {
            stream: true,
            ..Recording::fail(stage)
        }
    } else {
        Recording::fail(stage)
    };
    let error = if absolute {
        run_pointer_absolute(&mut s, &aim(true), &MouseButton::Left, deadline())
    } else {
        run_pointer(&mut s, &target(), &MouseButton::Left, deadline())
    }
    .expect_err(stage);
    let events = s.log;
    assert!(events.contains(&stage.to_owned()));
    if stage == "create" {
        assert_eq!(events, ["create"]);
    } else {
        assert_eq!(events.last().map(String::as_str), Some("stop"));
    }
    assert_eq!(
        error.code,
        if stage == "up" || stage == "stop" {
            ErrorCode::ActionNotSupported
        } else {
            ErrorCode::ActionFailed
        }
    );
    assert_eq!(
        error.disposition,
        match stage {
            "up" => DeliverySemantics::uncertain(),
            "stop" => DeliverySemantics::delivered_unverified(),
            _ => DeliverySemantics::unknown(),
        }
    );
    if stage == "up" || stage == "stop" {
        assert_eq!(error.details.unwrap()["physical_delivery_started"], true);
    }
}

#[test]
fn pointer_routing_and_failures() {
    let mut s = Recording {
        stream: true,
        ..Recording::default()
    };
    run_pointer_absolute(&mut s, &aim(true), &MouseButton::Left, deadline()).unwrap();
    assert_eq!(
        s.log,
        [
            "create", "attach", "start", "absolute", "down", "up", "stop"
        ]
    );
    assert_eq!(s.pos, (77., 21.));

    let mut s = Recording::default();
    run_pointer_absolute(&mut s, &aim(false), &MouseButton::Right, deadline()).unwrap();
    let events = s.log;
    assert_eq!(&events[..4], ["create", "attach", "stop", "create"]);
    assert_eq!(events.iter().filter(|e| e.as_str() == "down").count(), 1);
    assert_eq!(events.last().map(String::as_str), Some("stop"));

    for stage in ["create", "start", "motion", "down", "up", "stop"] {
        check(false, stage);
    }
    for stage in ["absolute", "down", "up", "stop"] {
        check(true, stage);
    }

    let mut modal = Recording::default();
    let e =
        run_pointer_absolute(&mut modal, &aim(true), &MouseButton::Left, deadline()).unwrap_err();
    assert_eq!(modal.log, ["create", "attach", "stop"]);
    assert_eq!(
        (e.code, e.disposition),
        (ErrorCode::ActionFailed, DeliverySemantics::not_delivered())
    );
    assert_eq!(e.details.unwrap()["physical_delivery_started"], false);
    let mut modal = Recording {
        stream: true,
        start_fails: true,
        ..Recording::default()
    };
    let e =
        run_pointer_absolute(&mut modal, &aim(true), &MouseButton::Left, deadline()).unwrap_err();
    assert_eq!(modal.log, ["create", "attach", "start", "stop"]);
    assert_eq!(e.disposition, DeliverySemantics::not_delivered());

    let spent = Deadline::after(1).unwrap();
    std::thread::sleep(Duration::from_millis(20));
    let mut s = Recording::default();
    run_pointer(&mut s, &target(), &MouseButton::Left, spent).unwrap();
    assert_eq!(s.recovery, [("up", true), ("stop", true)]);
}

fn display(id: &str, primary: bool, x: f64, y: f64) -> DisplayInfo {
    DisplayInfo {
        id: id.into(),
        bounds: Rect {
            x,
            y,
            width: 1280.,
            height: 800.,
        },
        is_primary: primary,
        scale: 1.,
    }
}
#[test]
fn monitor_routing() {
    let ds = [
        display("DP-1", true, 100., 50.),
        display("HDMI-1+DP-2", false, 1380., 50.),
    ];
    for (p, expected) in [
        (Point { x: 140., y: 70. }, ("DP-1", 40., 20.)),
        (Point { x: 1500., y: 70. }, ("HDMI-1", 120., 20.)),
    ] {
        let (c, q) = monitor_under(&p, &ds).unwrap();
        assert_eq!((c.as_str(), q.x, q.y), expected);
    }
    assert!(monitor_under(&Point { x: 1., y: 1. }, &ds).is_none());
    for y in [63., 64.] {
        assert_eq!(in_modal(&Point { x: 640., y }), y >= 64.);
    }
}
