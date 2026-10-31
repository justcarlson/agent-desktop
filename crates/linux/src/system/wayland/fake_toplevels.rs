//! The toplevel globals of the scripted compositor: an
//! `ext_foreign_toplevel_list_v1` that announces each fake toplevel with its
//! identifier, a `zwlr_foreign_toplevel_manager_v1` that announces each with
//! its title, state and parent and answers `activate` and `close`, and a
//! `hyprland_toplevel_mapping_manager_v1` that answers each mapping request
//! with the toplevel's address or `failed`.

use super::fake_compositor::{Arg, message};

pub(super) const LIST: &str = "ext_foreign_toplevel_list_v1";
pub(super) const MAPPER: &str = "hyprland_toplevel_mapping_manager_v1";
pub(super) const WLR_MANAGER: &str = "zwlr_foreign_toplevel_manager_v1";

const FIRST_SERVER_ID: u32 = 0xff00_0000;
const ACTIVATE: u16 = 4;
const CLOSE: u16 = 5;
const ACTIVATED: u32 = 2;

/// One toplevel the fake compositor announces. `address` is what its
/// mapping requests answer; `None` answers `failed`. `parent` is the index
/// of the toplevel it is a child of. A `stubborn` toplevel ignores
/// `activate` and `close`.
#[derive(Debug, Clone, Default)]
pub(crate) struct FakeToplevel {
    pub(crate) identifier: &'static str,
    pub(crate) address: Option<u64>,
    pub(crate) activated: bool,
    pub(crate) parent: Option<usize>,
    pub(crate) stubborn: bool,
    pub(crate) closed: bool,
}

/// The server-side ids of one connection's toplevel handles. A compositor
/// allocates them in order from `0xff000000`, so each protocol's handles
/// take the next run of ids when its global is bound.
#[derive(Debug, Clone, Copy)]
pub(super) struct HandleIds {
    next: u32,
    pub(super) ext: u32,
    pub(super) wlr: u32,
}

impl Default for HandleIds {
    fn default() -> Self {
        Self {
            next: FIRST_SERVER_ID,
            ext: 0,
            wlr: 0,
        }
    }
}

impl HandleIds {
    /// The first of `count` fresh ids.
    pub(super) fn take(&mut self, count: usize) -> u32 {
        let first = self.next;
        self.next += u32::try_from(count).expect("few toplevels");
        first
    }
}

fn at(first: u32, index: usize) -> u32 {
    first + u32::try_from(index).expect("few toplevels")
}

/// The list's `toplevel` event for each toplevel, then its `identifier`
/// and `done` events.
pub(super) fn list_events(list: u32, first: u32, toplevels: &[FakeToplevel]) -> Vec<u8> {
    let mut out = Vec::new();
    for (index, toplevel) in toplevels.iter().enumerate() {
        let id = at(first, index);
        out.extend(message(list, 0, &[Arg::Uint(id)]));
        out.extend(message(id, 4, &[Arg::Text(toplevel.identifier)]));
        out.extend(message(id, 1, &[]));
    }
    out
}

fn state_events(id: u32, toplevel: &FakeToplevel) -> Vec<u8> {
    let state: Vec<u8> = if toplevel.activated {
        ACTIVATED.to_ne_bytes().to_vec()
    } else {
        Vec::new()
    };
    let mut out = message(id, 4, &[Arg::Array(&state)]);
    out.extend(message(id, 5, &[]));
    out
}

/// The manager's `toplevel` event for each toplevel with its title and
/// state, then each child's `parent` event from version 3.
pub(super) fn manager_events(
    manager: (u32, u32),
    first: u32,
    toplevels: &[FakeToplevel],
) -> Vec<u8> {
    let (manager, version) = manager;
    let mut out = Vec::new();
    for (index, toplevel) in toplevels.iter().enumerate() {
        let id = at(first, index);
        out.extend(message(manager, 0, &[Arg::Uint(id)]));
        out.extend(message(id, 0, &[Arg::Text(toplevel.identifier)]));
        out.extend(state_events(id, toplevel));
    }
    for (index, toplevel) in toplevels.iter().enumerate() {
        if let (Some(parent), true) = (toplevel.parent, version >= 3) {
            let id = at(first, index);
            out.extend(message(id, 7, &[Arg::Uint(at(first, parent))]));
            out.extend(message(id, 5, &[]));
        }
    }
    out
}

/// The log entry for `request` on `toplevel`'s wlr handle.
pub(super) fn logged(request: u16, toplevel: &FakeToplevel) -> String {
    let name = match request {
        ACTIVATE => "activate",
        CLOSE => "close",
        _ => "request",
    };
    format!("{name} {}", toplevel.identifier)
}

/// The compositor's answer to `request` on the wlr handle at `index`.
pub(super) fn handle_request(
    (first, index): (u32, usize),
    request: u16,
    toplevels: &mut [FakeToplevel],
) -> Vec<u8> {
    if toplevels[index].stubborn || toplevels[index].closed {
        return Vec::new();
    }
    match request {
        ACTIVATE => {
            let mut out = Vec::new();
            for (each, toplevel) in toplevels.iter_mut().enumerate() {
                toplevel.activated = each == index;
                out.extend(state_events(at(first, each), toplevel));
            }
            out
        }
        CLOSE => {
            toplevels[index].closed = true;
            message(at(first, index), 6, &[])
        }
        _ => Vec::new(),
    }
}

fn address_reply(handle: u32, address: Option<u64>) -> Vec<u8> {
    match address {
        Some(address) => {
            let high = u32::try_from(address >> 32).expect("upper half");
            let low = u32::try_from(address & 0xffff_ffff).expect("lower half");
            message(handle, 0, &[Arg::Uint(high), Arg::Uint(low)])
        }
        None => message(handle, 1, &[]),
    }
}

/// The answer to a mapping request for the handle `toplevel` of the
/// protocol whose handles start at `first`.
pub(super) fn mapping_reply(
    handle: u32,
    (first, toplevel): (u32, u32),
    toplevels: &[FakeToplevel],
) -> Vec<u8> {
    let address = toplevel
        .checked_sub(first)
        .and_then(|index| toplevels.get(index as usize))
        .and_then(|toplevel| toplevel.address);
    address_reply(handle, address)
}
