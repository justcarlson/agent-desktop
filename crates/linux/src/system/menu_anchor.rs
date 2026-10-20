use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::time::Duration;

use agent_desktop_core::{AdapterError, Deadline, ProcessIdentity, WindowInfo};

use super::SharedBus;
use super::liveness::Liveness;
use super::menu_popup::Node;
use super::menu_reader::BusReader;
use super::menu_surface::{MenuPopup, PopupReader, find_popups, vanished};
use super::registry::read_registry_for;
use super::registry_app::RegistryApp;
use super::registry_frames::RootRead;
use super::signal_inventory::read_selected;
use super::window_id;
use crate::tree::AtspiElement;

enum Target {
    Window(WindowInfo),
    Root(AtspiElement),
}

fn owned_by(pid: u32) -> impl Fn(&RegistryApp) -> bool {
    move |app| app.root.pid() == pid
}

/// What to read for a process: its windows, each of which also reads the
/// top-level popups of the application. A process with no window is read from
/// its application roots, which is where a popup frame that is not a window
/// lives.
fn targets(roots: Vec<AtspiElement>, windows: Vec<WindowInfo>) -> Vec<Target> {
    if windows.is_empty() {
        roots.into_iter().map(Target::Root).collect()
    } else {
        windows.into_iter().map(Target::Window).collect()
    }
}

fn answer(
    state: Liveness,
    collect: impl FnOnce() -> Result<Vec<Target>, AdapterError>,
    mut read: impl FnMut(&Target) -> Result<bool, AdapterError>,
) -> Result<bool, AdapterError> {
    if state != Liveness::Alive {
        return Ok(false);
    }
    for target in collect()? {
        if read(&target)? {
            return Ok(true);
        }
    }
    Ok(false)
}

type TopLevels = RefCell<HashMap<String, (Vec<Node>, bool)>>;

/// What the reads of one poll share: whether the process is a web engine,
/// which the registry already says, and the top-level popup candidates of
/// each application, which depend on the application and not on the window
/// being read.
struct Poll<'a> {
    web: bool,
    top_levels: &'a TopLevels,
    incomplete: Cell<bool>,
}

struct AnchorReader<'a> {
    inner: BusReader<'a>,
    bus: &'a SharedBus,
    window: &'a AtspiElement,
    poll: &'a Poll<'a>,
    deadline: Deadline,
    top_incomplete: bool,
}

impl PopupReader for AnchorReader<'_> {
    fn window(&mut self) -> Result<Option<Node>, AdapterError> {
        if self.poll.web {
            return Ok(None);
        }
        self.inner.window()
    }

    fn top_levels(&mut self) -> Result<Vec<Node>, AdapterError> {
        let key = self.window.bus_name().to_owned();
        if !self.poll.top_levels.borrow().contains_key(&key) {
            let probe = AtspiElement::new(key.clone(), "/", self.window.pid());
            let mut reader = BusReader::new(self.bus, &probe, self.deadline);
            let all = reader.top_levels()?;
            self.poll
                .top_levels
                .borrow_mut()
                .insert(key.clone(), (all, reader.incomplete));
        }
        let cache = self.poll.top_levels.borrow();
        let (all, incomplete) = cache.get(&key).cloned().unwrap_or_default();
        self.top_incomplete |= incomplete;
        Ok(all
            .iter()
            .filter(|node| {
                let element = &node.payload.node.element;
                !(element.bus_name() == self.window.bus_name()
                    && element.path() == self.window.path())
            })
            .cloned()
            .collect())
    }

    fn pause(&mut self, gap: Duration) {
        self.inner.pause(gap);
    }
}

fn popups_at(
    bus: &SharedBus,
    window: &AtspiElement,
    poll: &Poll<'_>,
    deadline: Deadline,
) -> Result<Vec<MenuPopup>, AdapterError> {
    let mut reader = AnchorReader {
        inner: BusReader::new(bus, window, deadline),
        bus,
        window,
        poll,
        deadline,
        top_incomplete: false,
    };
    let found = find_popups(&mut reader, false, deadline);
    poll.incomplete
        .set(poll.incomplete.get() || reader.inner.incomplete || reader.top_incomplete);
    match found {
        Err(error) if vanished(&error) => Ok(Vec::new()),
        other => other,
    }
}

fn read_target(
    bus: &SharedBus,
    target: &Target,
    poll: &Poll<'_>,
    deadline: Deadline,
) -> Result<bool, AdapterError> {
    let popups = match target {
        Target::Window(window) => {
            let (bus_name, path) = window_id::decode(&window.id)?;
            let element = AtspiElement::new(bus_name, path, window.pid.get());
            popups_at(bus, &element, poll, deadline)?
        }
        Target::Root(root) => popups_at(bus, root, poll, deadline)?,
    };
    Ok(!popups.is_empty())
}

fn collect_targets(
    bus: &SharedBus,
    pid: u32,
    deadline: Deadline,
) -> Result<(Vec<Target>, bool, bool), AdapterError> {
    let registry = read_registry_for(bus, Some(pid), deadline)?;
    let owned = owned_by(pid);
    let (windows, outcomes) = read_selected(bus, &registry, &owned, deadline)?;
    let unread = outcomes
        .iter()
        .any(|outcome| outcome.read != RootRead::Read);
    let owners: Vec<&RegistryApp> = registry.apps.iter().filter(|app| owned(app)).collect();
    let web = owners.iter().any(|app| app.facts.web_family.is_some());
    let roots = owners.iter().map(|app| app.root.clone()).collect();
    let targets = targets(
        roots,
        windows.into_iter().map(|window| window.info).collect(),
    );
    Ok((targets, web, unread))
}

/// The answer of a poll that found no open menu: closed only when every read
/// that could have shown one was whole. A read that failed or was cut short
/// proves nothing about a menu it could not see, so the poll reports that it
/// could not tell and the wait goes on.
fn settle(open: bool, incomplete: bool) -> Result<bool, AdapterError> {
    if open || !incomplete {
        return Ok(open);
    }
    Err(AdapterError::timeout(
        "The menu state could not be read in full",
    ))
}

fn read_menu(
    bus: &SharedBus,
    process: &ProcessIdentity,
    state: Liveness,
    deadline: Deadline,
) -> Result<bool, AdapterError> {
    let pid = process.pid.get();
    let top_levels = TopLevels::default();
    let web = Cell::new(false);
    let unread = Cell::new(false);
    let poll_incomplete = Cell::new(false);
    let open = answer(
        state,
        || {
            let (found, is_web, not_whole) = collect_targets(bus, pid, deadline)?;
            web.set(is_web);
            unread.set(not_whole);
            Ok(found)
        },
        |target| {
            let poll = Poll {
                web: web.get(),
                top_levels: &top_levels,
                incomplete: Cell::new(false),
            };
            let seen = read_target(bus, target, &poll, deadline);
            poll_incomplete.set(poll_incomplete.get() || poll.incomplete.get());
            seen
        },
    )?;
    settle(open, unread.get() || poll_incomplete.get())
}

/// Whether the process has an open menu now. The caller checks that the
/// process is alive around each read: a process that is not alive answers
/// slowly or not at all, and the wait loop turns that state into its own
/// error.
pub(crate) fn menu_open(
    bus: &SharedBus,
    process: &ProcessIdentity,
    deadline: Deadline,
) -> Result<bool, AdapterError> {
    read_menu(bus, process, Liveness::Alive, deadline)
}

#[cfg(test)]
#[path = "menu_anchor_tests.rs"]
mod tests;
