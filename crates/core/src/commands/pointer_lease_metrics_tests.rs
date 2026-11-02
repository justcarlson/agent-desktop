use crate::adapter::{ActionOps, InputOps, ObservationOps, SystemOps};
use crate::commands::{
    drag::{self, DragArgs, DragEndpoint, WaitForScope},
    hover::{self, HoverArgs},
    mouse_click::{self, MouseClickArgs},
    mouse_move::{self, MouseMoveArgs},
    mouse_wheel::{self, MouseWheelArgs},
};
use crate::context::CommandContext;
use crate::{AdapterError, DragParams, MouseButton, MouseEvent, OperationReport};
use serde_json::{Value, json};
use std::time::Duration;

const DISPATCH_MS: u64 = 25;

struct ContendedLeaseAdapter {
    contention_count: u64,
    report_details: Option<Value>,
}

impl ContendedLeaseAdapter {
    fn contended(contention_count: u64) -> Self {
        Self {
            contention_count,
            report_details: None,
        }
    }

    fn report(&self) -> OperationReport {
        let mut report = crate::adapter::test_report();
        if let Some(details) = &self.report_details {
            report.result = report.result.with_details(details.clone());
        }
        report
    }
}

impl ObservationOps for ContendedLeaseAdapter {}
impl ActionOps for ContendedLeaseAdapter {}

impl SystemOps for ContendedLeaseAdapter {
    fn acquire_interaction_lease(
        &self,
        deadline: crate::Deadline,
    ) -> Result<crate::InteractionLease, AdapterError> {
        Ok(crate::InteractionLease::guarded_with_contention(
            deadline,
            (),
            self.contention_count,
        ))
    }
}

impl InputOps for ContendedLeaseAdapter {
    fn mouse_event(
        &self,
        _event: MouseEvent,
        _lease: &crate::InteractionLease,
    ) -> Result<OperationReport, AdapterError> {
        std::thread::sleep(Duration::from_millis(DISPATCH_MS));
        Ok(self.report())
    }

    fn drag(
        &self,
        _params: DragParams,
        _lease: &crate::InteractionLease,
    ) -> Result<OperationReport, AdapterError> {
        std::thread::sleep(Duration::from_millis(DISPATCH_MS));
        Ok(self.report())
    }
}

fn headed() -> CommandContext {
    CommandContext::default().with_headed(true)
}

fn click(adapter: &ContendedLeaseAdapter) -> Value {
    mouse_click::execute(
        MouseClickArgs {
            x: 10.0,
            y: 20.0,
            button: MouseButton::Left,
            count: 1,
            modifiers: Vec::new(),
        },
        adapter,
        &headed(),
    )
    .unwrap()
}

fn assert_lease_metrics(response: &Value, contention_count: u64) {
    let auto_wait = &response["details"]["auto_wait"];
    assert_eq!(
        auto_wait["lease_contention_count"],
        json!(contention_count),
        "{response}"
    );
    let hold_ms = auto_wait["lease_hold_ms"].as_u64();
    assert!(
        hold_ms.is_some_and(|hold_ms| hold_ms >= DISPATCH_MS),
        "lease_hold_ms must cover the dispatch under the lease: {response}"
    );
}

#[test]
fn mouse_click_reports_the_lease_contention_and_hold() {
    assert_lease_metrics(&click(&ContendedLeaseAdapter::contended(2)), 2);
}

#[test]
fn mouse_click_reads_the_contention_from_the_lease() {
    assert_lease_metrics(&click(&ContendedLeaseAdapter::contended(0)), 0);
    assert_lease_metrics(&click(&ContendedLeaseAdapter::contended(7)), 7);
}

#[test]
fn lease_metrics_keep_the_adapter_report_details() {
    let adapter = ContendedLeaseAdapter {
        contention_count: 2,
        report_details: Some(json!({ "delivered_events": 1 })),
    };
    let response = click(&adapter);
    assert_lease_metrics(&response, 2);
    assert_eq!(response["details"]["delivered_events"], json!(1));
}

#[test]
fn mouse_move_reports_the_lease_contention_and_hold() {
    let response = mouse_move::execute(
        MouseMoveArgs { x: 10.0, y: 20.0 },
        &ContendedLeaseAdapter::contended(2),
        &headed(),
    )
    .unwrap();
    assert_lease_metrics(&response, 2);
}

#[test]
fn mouse_wheel_reports_the_lease_contention_and_hold() {
    let response = mouse_wheel::execute(
        MouseWheelArgs {
            x: 10.0,
            y: 20.0,
            dy: 3.0,
            dx: 0.0,
            modifiers: Vec::new(),
        },
        &ContendedLeaseAdapter::contended(2),
        &headed(),
    )
    .unwrap();
    assert_lease_metrics(&response, 2);
}

#[test]
fn hover_by_coordinates_reports_the_lease_contention_and_hold() {
    let response = hover::execute(
        HoverArgs {
            ref_id: None,
            xy: Some((10.0, 20.0)),
            duration_ms: None,
            timeout_ms: None,
        },
        &ContendedLeaseAdapter::contended(2),
        &headed(),
    )
    .unwrap();
    assert_lease_metrics(&response, 2);
}

#[test]
fn drag_by_coordinates_reports_the_lease_contention_and_hold() {
    let response = drag::execute(
        DragArgs {
            from: DragEndpoint {
                ref_id: None,
                xy: Some((10.0, 20.0)),
            },
            to: DragEndpoint {
                ref_id: None,
                xy: Some((80.0, 20.0)),
            },
            duration_ms: None,
            drop_delay_ms: None,
            timeout_ms: None,
            wait_for_scope: WaitForScope::To,
        },
        &ContendedLeaseAdapter::contended(2),
        &headed(),
    )
    .unwrap();
    assert_lease_metrics(&response, 2);
}
