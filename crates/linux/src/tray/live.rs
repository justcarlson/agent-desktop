use agent_desktop_core::{
    AdapterError, Deadline, ElementState, LiveElement, LiveIdentity, LocatorField, Rect, state,
};

use super::menu_layout::LayoutEntry;
use super::node_evidence::{ITEM_ROLE, identifiers, item_states, name_field, presence};
use super::world::{LiveWorld, ShellWorld, shared_bus};
use super::{
    TrayTarget, call, capability, introspect, item_props, menu_layout, menu_nodes, menu_resolve,
};

pub(super) fn entry_state(entry: &LayoutEntry, role: &str, offscreen: bool) -> ElementState {
    let mut states = menu_nodes::states_of(entry);
    if !offscreen {
        states.retain(|token| token != state::OFFSCREEN);
    }
    ElementState {
        role: role.to_owned(),
        states,
        value: None,
        enabled: Some(entry.enabled),
        hidden: Some(false),
        offscreen: offscreen.then_some(true),
    }
}

pub(crate) fn element(
    target: &TrayTarget,
    deadline: Deadline,
) -> Result<LiveElement, AdapterError> {
    let connection = call::session(deadline)?;
    call::require_owner(&connection, &target.owner, deadline)?;
    let layout = target
        .menu_path
        .as_deref()
        .map(|menu| menu_layout::read(&connection, &target.owner, menu, deadline))
        .transpose()?;
    let entry = match (target.menu_path.as_deref(), layout.as_ref()) {
        (Some(_), Some(layout)) => Some(menu_resolve::find(layout, target)?),
        _ => None,
    };
    let props = item_props::read(&connection, &target.owner, &target.object_path, deadline)?;
    let (name, actions) = if let Some(entry) = entry {
        (entry.label.clone(), capability::entry_actions(entry))
    } else {
        let methods = introspect::item_methods(&call::introspect(
            &connection,
            &target.owner,
            &target.object_path,
            deadline,
        )?);
        (props.title.clone(), capability::actions(&methods))
    };
    let observed = LiveWorld::new(shared_bus()).panel_extents(&props.title, deadline);
    let (bounds, offscreen) = presence(observed);
    let element_state = if let Some(entry) = entry {
        entry_state(entry, menu_nodes::role_of(entry), offscreen)
    } else {
        let on_screen_states = if offscreen {
            vec![state::OFFSCREEN.to_owned()]
        } else {
            Vec::new()
        };
        ElementState {
            role: ITEM_ROLE.to_owned(),
            states: item_states(props.passive, on_screen_states),
            value: None,
            enabled: Some(true),
            hidden: Some(props.passive),
            offscreen: Some(offscreen),
        }
    };
    Ok(LiveElement {
        identity: LiveIdentity {
            name: name_field(&name),
            description: LocatorField::Absent,
            identifiers: identifiers(target),
        },
        state: element_state,
        states_complete: true,
        bounds,
        available_actions: actions,
    })
}

pub(crate) fn actions(
    target: &TrayTarget,
    deadline: Deadline,
) -> Result<Vec<String>, AdapterError> {
    element(target, deadline).map(|element| element.available_actions)
}

pub(crate) fn state(target: &TrayTarget, deadline: Deadline) -> Result<ElementState, AdapterError> {
    element(target, deadline).map(|element| element.state)
}

pub(crate) fn bounds(
    target: &TrayTarget,
    deadline: Deadline,
) -> Result<Option<Rect>, AdapterError> {
    element(target, deadline).map(|element| element.bounds)
}

#[cfg(test)]
#[path = "live_state_tests.rs"]
mod tests;
