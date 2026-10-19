use agent_desktop_core::{Action, ActionRequest, ActionResult, AdapterError, InteractionLease};

use super::activate::{self, BusWire, Item};
use super::world::{LiveWorld, shared_bus};
use super::{TrayTarget, call, introspect, item_props, menu_event};

pub(crate) fn execute(
    target: &TrayTarget,
    request: ActionRequest,
    lease: &InteractionLease,
) -> Result<ActionResult, AdapterError> {
    let deadline = lease.deadline();
    let connection = call::session(deadline)?;
    call::require_owner(&connection, &target.owner, deadline)?;
    if target.entry_id.is_some() {
        if !matches!(request.action, Action::Click) {
            return Err(activate::unsupported_action(
                target.label_path.last().map_or("", String::as_str),
                &request.action,
            ));
        }
        return menu_event::deliver(&connection, target, deadline);
    }
    let props = item_props::read(&connection, &target.owner, &target.object_path, deadline)?;
    let methods = introspect::item_methods(&call::introspect(
        &connection,
        &target.owner,
        &target.object_path,
        deadline,
    )?);
    let wire = BusWire {
        connection: &connection,
        target,
        deadline,
    };
    let world = LiveWorld::new(shared_bus());
    let item = Item {
        title: &props.title,
        methods: &methods,
        request: &request,
        deadline,
    };
    activate::run(&item, &wire, &world, lease)
}
