use agent_desktop_core::{
    Action, ActionRequest, ActionResult, AdapterError, DeliverySemantics, ErrorCode,
    InteractionLease,
};
use serde_json::json;

use crate::system::SharedBus;
use crate::tree::AtspiElement;

pub(crate) fn physical_delivery(
    bus: &SharedBus,
    action: &Action,
    element: Option<&AtspiElement>,
    request: &ActionRequest,
    lease: &InteractionLease,
) -> Result<ActionResult, AdapterError> {
    let _ = (bus, element, request, lease);
    Err(AdapterError::new(
        ErrorCode::ActionNotSupported,
        format!(
            "{} needs pointer or keyboard input, which this adapter does not deliver",
            action.name()
        ),
    )
    .with_suggestion("Use a semantic action such as click, set-value or type on a ref")
    .with_details(json!({
        "kind": "physical_delivery_unavailable",
        "physical_delivery_started": false,
    }))
    .with_disposition(DeliverySemantics::not_delivered()))
}

#[cfg(test)]
#[path = "physical_delivery_tests.rs"]
mod tests;
