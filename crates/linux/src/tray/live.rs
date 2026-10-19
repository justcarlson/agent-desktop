use agent_desktop_core::{
    AdapterError, Deadline, ElementState, LiveElement, LiveIdentity, LocatorField,
};

use super::node_evidence::{ITEM_ROLE, NO_GEOMETRY, identifiers, item_states, name_field};
use super::{
    TrayTarget, call, capability, introspect, item_props, menu_layout, menu_nodes, menu_resolve,
};

struct Read {
    name: String,
    role: &'static str,
    state: ElementState,
    actions: Vec<String>,
}

fn read_item(target: &TrayTarget, deadline: Deadline) -> Result<Read, AdapterError> {
    let connection = call::session(deadline)?;
    call::require_owner(&connection, &target.owner, deadline)?;
    let props = item_props::read(&connection, &target.owner, &target.object_path, deadline)?;
    let methods = introspect::item_methods(&call::introspect(
        &connection,
        &target.owner,
        &target.object_path,
        deadline,
    )?);
    Ok(Read {
        name: props.title,
        role: ITEM_ROLE,
        state: ElementState {
            role: ITEM_ROLE.to_owned(),
            states: item_states(props.passive, Vec::new()),
            value: None,
            enabled: Some(true),
            hidden: Some(props.passive),
            offscreen: None,
        },
        actions: capability::actions(&methods),
    })
}

fn read_entry(target: &TrayTarget, menu: &str, deadline: Deadline) -> Result<Read, AdapterError> {
    let connection = call::session(deadline)?;
    call::require_owner(&connection, &target.owner, deadline)?;
    let layout = menu_layout::read(&connection, &target.owner, menu, deadline)?;
    let entry = menu_resolve::find(&layout, target)?;
    let role = menu_nodes::role_of(entry);
    Ok(Read {
        name: entry.label.clone(),
        role,
        state: ElementState {
            role: role.to_owned(),
            states: menu_nodes::states_of(entry),
            value: None,
            enabled: Some(entry.enabled),
            hidden: Some(false),
            offscreen: None,
        },
        actions: capability::entry_actions(entry),
    })
}

fn read(target: &TrayTarget, deadline: Deadline) -> Result<Read, AdapterError> {
    match target.menu_path.as_deref() {
        Some(menu) => read_entry(target, menu, deadline),
        None => read_item(target, deadline),
    }
}

pub(super) fn actions(
    target: &TrayTarget,
    deadline: Deadline,
) -> Result<Vec<String>, AdapterError> {
    read(target, deadline).map(|read| read.actions)
}

pub(super) fn state(target: &TrayTarget, deadline: Deadline) -> Result<ElementState, AdapterError> {
    read(target, deadline).map(|read| read.state)
}

/// A tray node has no extents of its own. The rectangle it reports is empty
/// and positioned nowhere, which core's visibility gate accepts for a
/// semantic click, the only kind a tray node takes headless.
pub(super) fn element(
    target: &TrayTarget,
    deadline: Deadline,
) -> Result<LiveElement, AdapterError> {
    let read = read(target, deadline)?;
    Ok(LiveElement {
        identity: LiveIdentity {
            name: name_field(&read.name),
            description: LocatorField::Absent,
            identifiers: identifiers(target),
        },
        state: read.state,
        states_complete: true,
        bounds: Some(NO_GEOMETRY),
        available_actions: read.actions,
    })
}
