use std::time::Duration;

use agent_desktop_core::{
    AdapterError, Deadline, DeliverySemantics, DisplayInfo, ErrorCode, MouseButton, Point, Rect,
};

use super::{PointerSession, monitor_under, run_pointer};
use crate::input::wayland::key_session::KeySession;

#[derive(Default)]
struct Recording {
    log: Vec<String>,
    fail: Option<&'static str>,
    pos: (f64, f64),
    stream: bool,
}
impl Recording {
    fn step(&mut self, event: &str) -> Result<(), AdapterError> {
        self.log.push(event.into());
        let denied_down = self.fail == Some("denied_down") && event == "down";
        if self.fail == Some(event) || denied_down {
            return Err(if event == "up" || event == "stop" || denied_down {
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
        self.step("start")
    }
    fn keysym(&mut self, _: u32, _: bool, _: Deadline) -> Result<(), AdapterError> {
        Ok(())
    }
    fn stop(&mut self, _: Deadline) -> Result<(), AdapterError> {
        self.step("stop")
    }
}
impl PointerSession for Recording {
    fn button(&mut self, _: u32, down: bool, _: Deadline) -> Result<(), AdapterError> {
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
fn display(id: &str, x: f64, y: f64) -> DisplayInfo {
    DisplayInfo {
        id: id.into(),
        bounds: Rect {
            x,
            y,
            width: 1280.,
            height: 800.,
        },
        is_primary: true,
        scale: 1.,
    }
}

#[test]
fn missing_absolute_targeting_never_presses_a_shell_control() {
    let displays = [display("DP-1", 100., 0.)];
    for fail in [None, Some("start")] {
        let mut session = Recording {
            stream: fail.is_some(),
            fail,
            ..Recording::default()
        };
        let error = run_pointer(
            &mut session,
            &Point { x: 640., y: 16. },
            &displays,
            &MouseButton::Left,
            deadline(),
        )
        .expect_err("a guessed relative position cannot authorize a ref click");
        assert_eq!(error.disposition, DeliverySemantics::not_delivered());
        assert!(
            !session
                .log
                .iter()
                .any(|event| matches!(event.as_str(), "absolute" | "down"))
        );
        assert_eq!(session.log.last().map(String::as_str), Some("stop"));
    }
    let mut session = Recording {
        stream: true,
        ..Recording::default()
    };
    let error = run_pointer(
        &mut session,
        &Point { x: 70., y: 16. },
        &displays,
        &MouseButton::Left,
        deadline(),
    )
    .expect_err("a point in the monitor gap cannot authorize a click");
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
    assert!(session.log.is_empty());
}

#[test]
fn an_overlapping_control_with_an_off_display_center_is_not_aimed() {
    let displays = [display("DP-1", 100., 50.)];
    let error = crate::input::shell_aim::aim(
        Rect {
            x: 20.,
            y: 60.,
            width: 100.,
            height: 20.,
        },
        &displays,
    )
    .expect_err("the visible sliver does not prove the center is on a monitor");
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
}

#[test]
fn pointer_routing_and_failures() {
    let displays = [display("DP-1", 600., 50.)];
    let target = Point { x: 677., y: 71. };
    let mut session = Recording {
        stream: true,
        ..Recording::default()
    };
    run_pointer(
        &mut session,
        &target,
        &displays,
        &MouseButton::Left,
        deadline(),
    )
    .unwrap();
    assert_eq!(
        session.log,
        [
            "create", "attach", "start", "absolute", "down", "up", "stop"
        ]
    );
    assert_eq!(session.pos, (77., 21.));

    for stage in ["create", "absolute", "down", "up", "stop", "denied_down"] {
        let mut session = Recording {
            stream: true,
            fail: Some(stage),
            ..Recording::default()
        };
        let error = run_pointer(
            &mut session,
            &target,
            &displays,
            &MouseButton::Left,
            deadline(),
        )
        .unwrap_err();
        assert_eq!(
            session.log.last().map(String::as_str),
            Some(if stage == "create" { "create" } else { "stop" })
        );
        assert_eq!(
            error.disposition,
            match stage {
                "down" | "up" => DeliverySemantics::uncertain(),
                "stop" => DeliverySemantics::delivered_unverified(),
                "denied_down" => DeliverySemantics::not_delivered(),
                _ => DeliverySemantics::unknown(),
            }
        );
        if matches!(stage, "down" | "up" | "stop") {
            assert!(
                error
                    .suggestion
                    .as_deref()
                    .is_some_and(|s| s.contains("before retrying"))
            );
            assert_eq!(error.details.unwrap()["physical_delivery_started"], true);
        }
    }
}

#[test]
fn monitor_routing() {
    let ds = [
        display("DP-1", 100., 50.),
        display("HDMI-1+DP-2", 1380., 50.),
    ];
    for (p, expected) in [
        (Point { x: 140., y: 70. }, ("DP-1", 40., 20.)),
        (Point { x: 1500., y: 70. }, ("HDMI-1", 120., 20.)),
    ] {
        let (c, q) = monitor_under(&p, &ds).unwrap();
        assert_eq!((c.as_str(), q.x, q.y), expected);
    }
    assert!(monitor_under(&Point { x: 1., y: 1. }, &ds).is_none());
}
