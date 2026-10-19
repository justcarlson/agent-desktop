use std::sync::OnceLock;

use agent_desktop_core::{
    AdapterError, Deadline, ErrorCode, InteractionLease, InteractionPolicy, MouseButton, Point,
    Rect,
};

use crate::system::shell::container::{Landmark, MENU, locate};
use crate::system::shell::identity::find_root;
use crate::system::{SharedBus, within};
use crate::tree::{AtspiElement, list_children, read_node};

const PANEL_DEPTH: usize = 4;

/// What the tray code needs from the shell core, so its tests run on fakes.
pub(crate) trait ShellWorld {
    fn locate(&self, landmark: Landmark, deadline: Deadline) -> Result<AtspiElement, AdapterError>;
    fn click(
        &self,
        node: &AtspiElement,
        button: MouseButton,
        policy: InteractionPolicy,
        lease: &InteractionLease,
    ) -> Result<Point, AdapterError>;
    fn panel_node(&self, title: &str, deadline: Deadline) -> Result<AtspiElement, AdapterError>;
    fn panel_extents(&self, title: &str, deadline: Deadline) -> Option<Rect>;
}

pub(crate) fn shared_bus() -> &'static SharedBus {
    static BUS: OnceLock<SharedBus> = OnceLock::new();
    BUS.get_or_init(SharedBus::default)
}

pub(crate) struct LiveWorld<'a> {
    bus: &'a SharedBus,
}

impl<'a> LiveWorld<'a> {
    pub(crate) fn new(bus: &'a SharedBus) -> Self {
        Self { bus }
    }
}

fn no_panel_node(title: &str) -> AdapterError {
    AdapterError::new(
        ErrorCode::ElementNotFound,
        format!("The system tray shows no panel node titled '{title}'"),
    )
    .with_suggestion("Open the tray with 'open-surface system-tray', then retry")
}

fn ambiguous_panel_node(title: &str) -> AdapterError {
    AdapterError::ambiguous_target(format!(
        "The system tray shows several panel nodes titled '{title}', so the pointer cannot tell which is the item"
    ))
}

/// The one panel node that carries `title`. A repeated title names no node,
/// because the panel order is not evidence of which item a node is.
pub(crate) fn choose_panel_node(names: &[String], title: &str) -> Result<usize, AdapterError> {
    let mut matching = names
        .iter()
        .enumerate()
        .filter(|(_, name)| name.as_str() == title)
        .map(|(index, _)| index);
    match (matching.next(), matching.next()) {
        (Some(index), None) => Ok(index),
        (None, _) => Err(no_panel_node(title)),
        (Some(_), Some(_)) => Err(ambiguous_panel_node(title)),
    }
}

impl ShellWorld for LiveWorld<'_> {
    fn locate(&self, landmark: Landmark, deadline: Deadline) -> Result<AtspiElement, AdapterError> {
        let root = find_root(self.bus, deadline)?;
        locate(self.bus, &root, landmark, deadline)
    }

    fn click(
        &self,
        node: &AtspiElement,
        button: MouseButton,
        policy: InteractionPolicy,
        lease: &InteractionLease,
    ) -> Result<Point, AdapterError> {
        crate::input::shell_click::click(self.bus, node, button, policy, lease)
    }

    fn panel_node(&self, title: &str, deadline: Deadline) -> Result<AtspiElement, AdapterError> {
        self.panel_read(title, deadline).map(|(node, _)| node)
    }

    fn panel_extents(&self, title: &str, deadline: Deadline) -> Option<Rect> {
        self.panel_read(title, deadline).ok()?.1
    }
}

impl LiveWorld<'_> {
    fn panel_read(
        &self,
        title: &str,
        deadline: Deadline,
    ) -> Result<(AtspiElement, Option<Rect>), AdapterError> {
        let panel = self.locate(Landmark::TrayPanel, deadline)?;
        let connection = self.bus.get(deadline)?;
        let (children, names, extents) = within(deadline, async {
            let mut children = Vec::new();
            let mut names = Vec::new();
            let mut extents = Vec::new();
            let mut pending: Vec<(AtspiElement, usize)> = vec![(panel.clone(), 0)];
            while let Some((parent, depth)) = pending.pop() {
                if depth >= PANEL_DEPTH {
                    continue;
                }
                let addresses = list_children(connection.inner(), &parent).await?;
                for (bus_name, path) in addresses.into_iter().rev() {
                    let child = AtspiElement::new(bus_name, path, panel.pid());
                    let node = read_node(connection.inner(), &child).await?;
                    if node.traits.role.present() == Some(&MENU) {
                        names.push(node.properties.name.present().cloned().unwrap_or_default());
                        extents.push(node.extents.present().copied().map(Rect::from));
                        children.push(child);
                    } else {
                        pending.push((child, depth + 1));
                    }
                }
            }
            Ok((children, names, extents))
        })?;
        let index = choose_panel_node(&names, title)?;
        let node = children
            .into_iter()
            .nth(index)
            .ok_or_else(|| no_panel_node(title))?;
        Ok((node, extents.get(index).copied().flatten()))
    }
}

#[cfg(test)]
#[path = "world_tests.rs"]
mod tests;
