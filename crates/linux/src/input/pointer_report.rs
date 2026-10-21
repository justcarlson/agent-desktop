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
