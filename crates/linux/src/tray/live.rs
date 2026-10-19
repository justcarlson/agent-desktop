use agent_desktop_core::{
    AdapterError, Deadline, ElementState, LiveElement, LiveIdentity, LocatorField, Rect, state,
};

use super::node_evidence::{ITEM_ROLE, identifiers, item_states, name_field, presence};
use super::world::{LiveWorld, ShellWorld, shared_bus};
use super::{
    TrayTarget, call, capability, introspect, item_props, menu_layout, menu_nodes, menu_resolve,
};

struct Read {
    name: String,
    state: ElementState,
    actions: Vec<String>,
    bounds: Option<Rect>,
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
    let observed = LiveWorld::new(shared_bus()).panel_extents(&props.title, deadline);
    let (bounds, offscreen) = presence(observed);
    let on_screen_states = if offscreen {
        vec![state::OFFSCREEN.to_owned()]
    } else {
        Vec::new()
    };
    Ok(Read {
        name: props.title,
        state: ElementState {
            role: ITEM_ROLE.to_owned(),
            states: item_states(props.passive, on_screen_states),
            value: None,
            enabled: Some(true),
            hidden: Some(props.passive),
            offscreen: Some(offscreen),
        },
        actions: capability::actions(&methods),
        bounds,
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
        state: ElementState {
            role: role.to_owned(),
            states: menu_nodes::states_of(entry),
            value: None,
            enabled: Some(entry.enabled),
            hidden: Some(false),
            offscreen: Some(true),
        },
        actions: capability::entry_actions(entry),
        bounds: None,
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

/// A bound item reports the extents of the shell panel node that shows it. An
/// item with no panel node and a menu entry have no on-screen presence, so
/// they report no rectangle and are marked offscreen.
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
        bounds: read.bounds,
        available_actions: read.actions,
    })
}

pub(super) fn bounds(
    target: &TrayTarget,
    deadline: Deadline,
) -> Result<Option<Rect>, AdapterError> {
    read(target, deadline).map(|read| read.bounds)
}
