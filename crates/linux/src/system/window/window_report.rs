use agent_desktop_core::{OperationReport, WindowOp, WindowReadback};
use serde_json::{Map, Value, json};

use super::window_op_verdict::POSITION_TOLERANCE;
use crate::system::x11::window_facts::WindowFacts;

/// What a successful window operation read back before it answered. A
/// restore is left unverified because its branches can lift a minimized
/// window, clear maximization, return a saved place, or change nothing; no
/// single readback scope describes every successful restore.
pub(in crate::system) fn window_readback(op: &WindowOp) -> Option<WindowReadback> {
    match op {
        WindowOp::Resize { .. } => Some(WindowReadback::geometry(
            "window_size",
            f64::from(POSITION_TOLERANCE),
            "pixels",
        )),
        WindowOp::Move { .. } => Some(WindowReadback::geometry(
            "window_position",
            f64::from(POSITION_TOLERANCE),
            "pixels",
        )),
        WindowOp::Minimize => Some(WindowReadback::state("window_minimized_state")),
        WindowOp::Maximize => Some(WindowReadback::state("window_maximized_state")),
        WindowOp::Restore => None,
    }
}

/// The window as drawn when a resize was read back: where the manager left
/// it and the size it holds, in the rectangle `list-windows` reports.
pub(in crate::system) fn visible_observation(facts: &WindowFacts) -> Value {
    let drawn = facts.visible();
    json!({ "x": drawn.x, "y": drawn.y, "width": drawn.width, "height": drawn.height })
}

/// The report with what the operation read back added to its details.
pub(in crate::system) fn with_observed(
    mut report: OperationReport,
    observed: Option<Value>,
) -> OperationReport {
    let Some(observed) = observed else {
        return report;
    };
    let mut details = match report.result.details.take() {
        Some(Value::Object(details)) => details,
        _ => Map::new(),
    };
    details.insert("observed".to_owned(), observed);
    report.result.details = Some(Value::Object(details));
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_desktop_core::{DeliverySemantics, OperationTarget, ProcessIdentity};

    fn report(op: &WindowOp) -> OperationReport {
        let target = OperationTarget::Window {
            id: "w-1".into(),
            title: "Doc".into(),
            process: ProcessIdentity::new(7u32, "boot"),
        };
        OperationReport::window_operation(target, op, window_readback(op))
    }

    #[test]
    fn a_restore_stays_unverified_without_a_uniform_readback_scope() {
        assert_eq!(
            report(&WindowOp::Restore).disposition(),
            DeliverySemantics::delivered_unverified()
        );
    }

    #[test]
    fn geometry_names_its_scope_with_the_real_tolerance_in_pixels() {
        let moved = report(&WindowOp::Move { x: 1.0, y: 2.0 });
        let details = moved.result.details.unwrap();
        assert_eq!(details["verification_scope"], "window_position");
        assert_eq!(details["units"], "pixels");
    }

    fn facts(shadow: (i32, i32, i32, i32)) -> WindowFacts {
        WindowFacts {
            client: crate::tree::Extents {
                x: 386,
                y: 189,
                width: 832,
                height: 628,
            },
            shadow,
            hidden: false,
            maximized_vertical: false,
            maximized_horizontal: false,
        }
    }

    #[test]
    fn the_observation_is_the_window_as_drawn_without_its_shadow() {
        assert_eq!(
            visible_observation(&facts((14, 12, 14, 17))),
            json!({ "x": 400, "y": 201, "width": 804, "height": 599 })
        );
        assert_eq!(
            visible_observation(&facts((0, 0, 0, 0))),
            json!({ "x": 386, "y": 189, "width": 832, "height": 628 })
        );
    }

    #[test]
    fn an_observation_joins_the_scope_and_the_tolerance_in_the_details() {
        let observed = visible_observation(&facts((14, 12, 14, 17)));
        let details = with_observed(
            report(&WindowOp::Resize {
                width: 1.0,
                height: 1.0,
            }),
            Some(observed.clone()),
        )
        .result
        .details
        .unwrap();
        assert_eq!(details["verification_scope"], "window_size");
        assert_eq!(details["tolerance"], 2.0);
        assert_eq!(details["observed"], observed);
    }

    #[test]
    fn no_observation_leaves_the_details_as_they_were() {
        let op = WindowOp::Move { x: 1.0, y: 2.0 };
        let details = with_observed(report(&op), None).result.details.unwrap();
        assert!(details.get("observed").is_none());
        assert_eq!(details["verification_scope"], "window_position");
    }

    #[test]
    fn minimize_and_maximize_report_a_state_scope_without_a_tolerance() {
        for (op, scope) in [
            (WindowOp::Minimize, "window_minimized_state"),
            (WindowOp::Maximize, "window_maximized_state"),
        ] {
            let details = report(&op).result.details.unwrap();
            assert_eq!(details["verification_scope"], scope);
            assert!(details.get("tolerance").is_none());
        }
    }
}
