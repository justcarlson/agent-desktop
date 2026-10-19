use agent_desktop_core::{
    Action, ActionRequest, ActionResult, ActionStep, AdapterError, Deadline, DeliverySemantics,
    ErrorCode, InteractionLease, InteractionPolicy, MouseButton, StepMechanism,
};
use zbus::Connection;

use super::TrayTarget;
use super::call;
use super::world::ShellWorld;

const ITEM_INTERFACE: &str = "org.kde.StatusNotifierItem";

#[derive(Debug)]
pub(crate) enum Route {
    Method(&'static str),
    Pointer(MouseButton),
    Refuse,
}

/// What one tray item action sends. `Activate` and `ContextMenu` are used
/// only when the item lists them. Without them the click goes to the panel
/// node with the pointer, and only where the policy lets the cursor move.
/// `SecondaryActivate` is the middle click, a different action, so no route
/// here ever names it.
pub(crate) fn route(action: &Action, methods: &[String], policy: InteractionPolicy) -> Route {
    let lists = |name: &str| methods.iter().any(|method| method == name);
    let (method, button) = match action {
        Action::Click => ("Activate", MouseButton::Left),
        Action::RightClick => ("ContextMenu", MouseButton::Right),
        _ => return Route::Refuse,
    };
    if lists(method) {
        Route::Method(method)
    } else if policy.allow_cursor_move {
        Route::Pointer(button)
    } else {
        Route::Refuse
    }
}

pub(crate) fn refusal(title: &str, action: &Action) -> AdapterError {
    let (verb, method) = match action {
        Action::RightClick => ("right-click", "ContextMenu"),
        _ => ("click", "Activate"),
    };
    AdapterError::new(
        ErrorCode::ActionNotSupported,
        format!("The tray item '{title}' lists no {method} method, so a headless {verb} cannot be delivered"),
    )
    .with_suggestion("Retry with --headed to use the pointer, or use the item's menu entries")
    .with_details(serde_json::json!({ "physical_delivery_started": false }))
    .with_disposition(DeliverySemantics::not_delivered())
}

pub(crate) fn unsupported_action(title: &str, action: &Action) -> AdapterError {
    AdapterError::new(
        ErrorCode::ActionNotSupported,
        format!(
            "The tray item '{title}' supports click and right-click, not {}",
            action.name()
        ),
    )
    .with_details(serde_json::json!({ "physical_delivery_started": false }))
    .with_disposition(DeliverySemantics::not_delivered())
}

/// What an item action needs from the bus, so its rules run on a recording
/// fake.
pub(crate) trait ItemWire {
    fn owner_present(&self) -> Result<bool, AdapterError>;
    fn send(&self, method: &str) -> Result<(), AdapterError>;
}

pub(crate) struct BusWire<'a> {
    pub(crate) connection: &'a Connection,
    pub(crate) target: &'a TrayTarget,
    pub(crate) deadline: Deadline,
}

impl ItemWire for BusWire<'_> {
    fn owner_present(&self) -> Result<bool, AdapterError> {
        call::owner_alive(self.connection, &self.target.owner, self.deadline)
    }

    fn send(&self, method: &str) -> Result<(), AdapterError> {
        call::call(
            self.connection,
            &self.target.owner,
            &self.target.object_path,
            ITEM_INTERFACE,
            method,
            &(0i32, 0i32),
            self.deadline,
        )
        .map(|_| ())
    }
}

pub(crate) struct Item<'a> {
    pub(crate) title: &'a str,
    pub(crate) methods: &'a [String],
    pub(crate) request: &'a ActionRequest,
    pub(crate) deadline: Deadline,
}

fn step(label: &'static str, mechanism: StepMechanism) -> ActionStep {
    ActionStep::succeeded(label).with_mechanism(mechanism)
}

pub(crate) fn run(
    item: &Item<'_>,
    wire: &dyn ItemWire,
    world: &dyn ShellWorld,
    lease: &InteractionLease,
) -> Result<ActionResult, AdapterError> {
    let action = &item.request.action;
    if !matches!(action, Action::Click | Action::RightClick) {
        return Err(unsupported_action(item.title, action));
    }
    let route = route(action, item.methods, item.request.policy);
    if matches!(route, Route::Refuse) {
        return Err(refusal(item.title, action));
    }
    if !wire.owner_present()? {
        return Err(call::owner_gone());
    }
    let step = match route {
        Route::Method(method) => {
            wire.send(method)?;
            step("sni-method", StepMechanism::SemanticApi)
        }
        Route::Pointer(button) => {
            let node = world.panel_node(item.title, item.deadline)?;
            world.click(&node, button, item.request.policy, lease)?;
            step("panel-pointer", StepMechanism::PhysicalSynthetic)
        }
        Route::Refuse => return Err(refusal(item.title, action)),
    };
    Ok(ActionResult::from_execution(action, vec![step]))
}

#[cfg(test)]
#[path = "activate_tests.rs"]
mod tests;
