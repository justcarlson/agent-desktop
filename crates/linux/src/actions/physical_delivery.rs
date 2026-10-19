use agent_desktop_core::{Action, ActionRequest, ActionResult, AdapterError, InteractionLease};

use crate::system::SharedBus;
use crate::tree::AtspiElement;

pub(crate) fn physical_delivery(
    bus: &SharedBus,
    action: &Action,
    element: Option<&AtspiElement>,
    request: &ActionRequest,
    lease: &InteractionLease,
) -> Result<ActionResult, AdapterError> {
    super::physical_target::deliver(bus, action, element, request, lease)
}
