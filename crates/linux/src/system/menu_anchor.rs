use std::collections::HashMap;
use std::time::Duration;

use agent_desktop_core::{AdapterError, Deadline, ProcessIdentity, WindowInfo};

use super::SharedBus;
use super::menu_popup::Node;
use super::menu_reader::BusReader;
use super::menu_surface::{PopupReader, find_popups, vanished};
use super::registry::read_registry_for;
use super::registry_app::RegistryApp;
use super::registry_frames::RootRead;
use super::signal_inventory::read_selected;
use super::window_id;
use crate::tree::AtspiElement;

fn owned_by(pid: u32) -> impl Fn(&RegistryApp) -> bool {
    move |app| app.root.pid() == pid
}

/// Prefer each process window; a process without windows is read from its
/// application roots, where a popup frame may live without being a window.
fn targets(
    roots: Vec<AtspiElement>,
    windows: Vec<WindowInfo>,
) -> Result<Vec<AtspiElement>, AdapterError> {
    if windows.is_empty() {
        Ok(roots)
    } else {
        windows
            .into_iter()
            .map(|window| {
                let (bus_name, path) = window_id::decode(&window.id)?;
                Ok(AtspiElement::new(bus_name, path, window.pid.get()))
            })
            .collect()
    }
}

/// The reads of one poll share the per-application top-level cache and the
/// incomplete-read bit. Both are mutated by the single-threaded read path.
struct Poll {
    web: bool,
    top_levels: HashMap<String, (Vec<Node>, bool)>,
    incomplete: bool,
}

struct AnchorReader<'a> {
    inner: BusReader<'a>,
    poll: &'a mut Poll,
}

impl PopupReader for AnchorReader<'_> {
    fn window(&mut self) -> Result<Option<Node>, AdapterError> {
        if self.poll.web {
            return Ok(None);
        }
        self.inner.window()
    }

    fn top_levels(&mut self) -> Result<Vec<Node>, AdapterError> {
        let key = self.inner.window.bus_name().to_owned();
        if !self.poll.top_levels.contains_key(&key) {
            let probe = AtspiElement::new(key.clone(), "/", self.inner.window.pid());
            let mut reader = BusReader::new(self.inner.bus, &probe, self.inner.deadline);
            let all = reader.top_levels()?;
            self.poll
                .top_levels
                .insert(key.clone(), (all, reader.incomplete));
        }
        let (all, incomplete) = self.poll.top_levels.get(&key).cloned().unwrap_or_default();
        self.poll.incomplete |= incomplete;
        Ok(all
            .into_iter()
            .filter(|node| {
                let element = &node.payload.node.element;
                !(element.bus_name() == self.inner.window.bus_name()
                    && element.path() == self.inner.window.path())
            })
            .collect())
    }

    fn pause(&mut self, gap: Duration) {
        self.inner.pause(gap);
    }
}

fn popups_at(
    bus: &SharedBus,
    window: &AtspiElement,
    poll: &mut Poll,
    deadline: Deadline,
) -> Result<bool, AdapterError> {
    let inner = BusReader::new(bus, window, deadline);
    let mut reader = AnchorReader { inner, poll };
    let found = find_popups(&mut reader, false, deadline);
    reader.poll.incomplete |= reader.inner.incomplete;
    match found {
        Ok(popups) => Ok(!popups.is_empty()),
        Err(error) if vanished(&error) => Ok(false),
        Err(error) => Err(error),
    }
}

fn collect_targets(
    bus: &SharedBus,
    pid: u32,
    deadline: Deadline,
) -> Result<(Vec<AtspiElement>, bool, bool), AdapterError> {
    let registry = read_registry_for(bus, Some(pid), deadline)?;
    let owned = owned_by(pid);
    let (windows, outcomes) = read_selected(bus, &registry, &owned, deadline)?;
    let unread = outcomes
        .iter()
        .any(|outcome| outcome.read != RootRead::Read);
    let owners: Vec<&RegistryApp> = registry.apps.iter().filter(|app| owned(app)).collect();
    let web = owners.iter().any(|app| app.facts.web_family.is_some());
    let roots = owners.iter().map(|app| app.root.clone()).collect();
    Ok((
        targets(
            roots,
            windows.into_iter().map(|window| window.info).collect(),
        )?,
        web,
        unread,
    ))
}

/// A closed answer is valid only when every read that could have shown a menu
/// was whole. An incomplete read proves nothing about an unseen menu.
fn settle(incomplete: bool) -> Result<bool, AdapterError> {
    if !incomplete {
        return Ok(false);
    }
    Err(AdapterError::timeout(
        "The menu state could not be read in full",
    ))
}

/// Whether the process has an open menu now. The wait loop checks process
/// liveness around each read and maps a stopped process to its own error.
pub(crate) fn menu_open(
    bus: &SharedBus,
    process: &ProcessIdentity,
    deadline: Deadline,
) -> Result<bool, AdapterError> {
    let (targets, web, unread) = collect_targets(bus, process.pid.get(), deadline)?;
    let mut poll = Poll {
        web,
        top_levels: HashMap::new(),
        incomplete: unread,
    };
    for target in targets {
        if popups_at(bus, &target, &mut poll, deadline)? {
            return Ok(true);
        }
    }
    settle(poll.incomplete)
}

#[cfg(test)]
#[path = "menu_anchor_tests.rs"]
mod tests;
