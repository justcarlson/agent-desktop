use agent_desktop_core::OperationReport;
use serde_json::json;

use super::delivery::Delivery;

pub(crate) fn with_event_count(report: OperationReport, delivery: &Delivery) -> OperationReport {
    OperationReport {
        result: report
            .result
            .with_details(json!({ "delivered_events": delivery.delivered_events })),
        target: report.target,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_desktop_core::{DeliverySemantics, DragParams, Point};

    #[test]
    fn emitted_event_count_is_kept_without_claiming_an_effect() {
        let report = OperationReport::drag_dispatched(&DragParams {
            from: Point { x: 0.0, y: 0.0 },
            to: Point { x: 4.0, y: 4.0 },
            duration_ms: None,
            drop_delay_ms: None,
        });

        let counted = with_event_count(
            report,
            &Delivery {
                delivered_events: 7,
            },
        );

        assert_eq!(
            counted.result.details.as_ref().unwrap()["delivered_events"],
            7
        );
        assert_eq!(
            counted.disposition(),
            DeliverySemantics::delivered_unverified()
        );
    }
}
