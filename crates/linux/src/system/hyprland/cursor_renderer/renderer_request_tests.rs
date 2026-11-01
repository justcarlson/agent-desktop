use agent_desktop_core::{
    CursorOverlayConfig, CursorOverlayControl, CursorOverlayInstruction, CursorOverlayStyle, Point,
};

use super::RendererRequest;

fn present() -> CursorOverlayControl {
    let config = CursorOverlayConfig::enabled(None, 6).expect("config");
    let instruction =
        CursorOverlayInstruction::new(Point { x: 10.0, y: 20.0 }, &config, true).expect("instr");
    CursorOverlayControl::present("s1".into(), instruction)
}

fn round_trip(request: &RendererRequest) -> RendererRequest {
    let json = request.to_json().expect("encode");
    RendererRequest::parse(json.to_string().as_bytes()).expect("decode")
}

#[test]
fn every_message_round_trips_under_its_frozen_name() {
    let cases = [
        (
            RendererRequest::control(CursorOverlayControl::enable(
                "s1".into(),
                CursorOverlayStyle::default(),
            )),
            "enable",
        ),
        (
            RendererRequest::Control {
                control: Box::new(present()),
                park: true,
            },
            "present",
        ),
        (
            RendererRequest::control(CursorOverlayControl::disable("s1".into())),
            "disable",
        ),
        (
            RendererRequest::Hide {
                holder: "n1".into(),
                expiry_ms: 500,
            },
            "hide",
        ),
        (
            RendererRequest::Show {
                holder: "n1".into(),
            },
            "show",
        ),
        (RendererRequest::Status, "status"),
    ];
    for (request, name) in cases {
        assert_eq!(request.message(), name);
        assert_eq!(request.to_json().expect("encode")["message"], name);
        assert_eq!(round_trip(&request), request);
    }
}

#[test]
fn present_carries_park_and_others_do_not() {
    let parked = RendererRequest::Control {
        control: Box::new(present()),
        park: true,
    };
    assert_eq!(parked.to_json().expect("encode")["park"], true);
    let enable = RendererRequest::control(CursorOverlayControl::disable("s1".into()));
    assert!(enable.to_json().expect("encode").get("park").is_none());
}

#[test]
fn a_message_that_does_not_match_its_control_is_refused() {
    let control = serde_json::to_value(CursorOverlayControl::disable("s1".into())).expect("json");
    let wire = serde_json::json!({"message": "enable", "control": control});
    assert!(RendererRequest::parse(wire.to_string().as_bytes()).is_err());
    assert!(RendererRequest::parse(br#"{"message":"hide"}"#).is_err());
}
