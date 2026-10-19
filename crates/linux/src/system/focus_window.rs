use std::time::Duration;

use agent_desktop_core::{AdapterError, Deadline, InteractionLease, WindowInfo};

use super::process_facts::Lookup;
use super::x11::activate::ACTIVATION_GRACE;
use super::x11::activate_error::ActivateError;
use super::x11::map::{ClientMatch, match_client};
use super::x11::stacking_read::StackingRead;
use super::x11::{FrameFacts, X11State};
use super::{SharedBus, window_id};
use crate::tree::Extents;

use super::focus_cause::{Cause, refusal};
use super::focus_world;

const POLL: Duration = Duration::from_millis(25);

/// Which display server the session uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Session {
    X11,
    Wayland,
    NoDisplay,
}

/// What the window's own frame says about itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct FrameView {
    pub(super) minimized: Option<bool>,
    pub(super) active: Option<bool>,
}

/// Everything `focus_window` reads and writes, so the decision can be driven
/// by a fixed desktop.
pub(super) trait FocusWorld {
    fn session(&self) -> Session;
    fn lookup(&self, pid: u32) -> Lookup;
    fn frame(&self) -> Result<FrameView, AdapterError>;
    fn clients(&self) -> X11State;
    fn stacking(&self, client: u32) -> StackingRead;
    fn global_position(&self) -> bool;
    fn send(&self, client: u32) -> Result<(), ActivateError>;
    fn active(&self) -> Option<u32>;
}

fn check_process(world: &impl FocusWorld, win: &WindowInfo) -> Result<(), AdapterError> {
    match world.lookup(win.pid.get()) {
        Lookup::Found(facts) if win.process_instance.as_deref() != Some(&facts.instance) => Err(
            AdapterError::stale_ref_because("The window belongs to a different process instance"),
        ),
        Lookup::Gone => Err(AdapterError::stale_ref_because(
            "The window's process has exited",
        )),
        Lookup::Found(_) | Lookup::Unreadable => Ok(()),
    }
}

fn facts_of(win: &WindowInfo, global_position: bool) -> FrameFacts {
    FrameFacts {
        pid: win.pid.get(),
        title: win.title.clone(),
        extents: win.bounds.map(|rect| Extents {
            x: rect.x as i32,
            y: rect.y as i32,
            width: rect.width as i32,
            height: rect.height as i32,
        }),
        global_position,
    }
}

fn client_of(
    world: &impl FocusWorld,
    win: &WindowInfo,
    clients: &[super::x11::X11Client],
) -> Result<u32, Cause> {
    let mut found = match_client(&facts_of(win, false), clients);
    if matches!(found, ClientMatch::Several(_)) {
        found = match_client(&facts_of(win, world.global_position()), clients);
    }
    match found {
        ClientMatch::One(id) => Ok(id),
        ClientMatch::Several(_) => Err(Cause::AmbiguousWindow),
        ClientMatch::NoClient => Err(Cause::WindowNotMapped),
    }
}

fn check_visible(world: &impl FocusWorld, client: u32) -> Result<(), Cause> {
    let StackingRead::Ready(snapshot) = world.stacking(client) else {
        return Err(Cause::X11Unavailable);
    };
    let window = snapshot
        .stack
        .iter()
        .find(|window| window.id == client)
        .ok_or(Cause::X11Unavailable)?;
    if window.flags.hidden {
        Err(Cause::WindowHidden)
    } else if !window.flags.mapped {
        Err(Cause::WindowNotMapped)
    } else {
        Ok(())
    }
}

fn is_active(world: &impl FocusWorld, client: u32) -> (Option<u32>, bool) {
    let active = world.active();
    let frame = world.frame().ok().and_then(|view| view.active);
    (active, active == Some(client) && frame == Some(true))
}

fn timeout_cause(before: Option<u32>, after: Option<u32>, client: u32) -> Cause {
    match (before, after) {
        (_, Some(now)) if now == client => Cause::FrameNotActive,
        (None, None) => Cause::ActivationNotGranted,
        _ => Cause::ActivationRefused,
    }
}

fn wait_for_focus(
    world: &impl FocusWorld,
    client: u32,
    before: Option<u32>,
    deadline: Deadline,
) -> Result<(), AdapterError> {
    loop {
        let (active, done) = is_active(world, client);
        if done {
            return Ok(());
        }
        if deadline.is_expired() {
            return Err(refusal(timeout_cause(before, active, client)));
        }
        std::thread::sleep(POLL.min(deadline.remaining()));
    }
}

/// Makes the window active through the window manager and returns only when
/// the display server and the window's own frame both say it is.
pub(super) fn run(
    world: &impl FocusWorld,
    win: &WindowInfo,
    deadline: Deadline,
) -> Result<(), AdapterError> {
    window_id::decode(&win.id).map_err(|error| {
        error.with_disposition(agent_desktop_core::DeliverySemantics::not_delivered())
    })?;
    check_process(world, win)?;
    match world.session() {
        Session::X11 => {}
        Session::Wayland => return Err(refusal(Cause::WaylandSession)),
        Session::NoDisplay => return Err(refusal(Cause::NoDisplay)),
    }
    let frame = world.frame()?;
    if frame.minimized == Some(true) {
        return Err(refusal(Cause::WindowMinimized));
    }
    let (active, clients) = match world.clients() {
        X11State::Ready { active, clients } => (active, clients),
        X11State::NoWindowManager => return Err(refusal(Cause::NoWindowManager)),
        X11State::Unavailable => return Err(refusal(Cause::X11Unavailable)),
    };
    let client = client_of(world, win, &clients).map_err(refusal)?;
    check_visible(world, client).map_err(refusal)?;
    if active == Some(client) && frame.active == Some(true) {
        return Ok(());
    }
    world
        .send(client)
        .map_err(|_| refusal(Cause::ActivationNotSent))?;
    wait_for_focus(world, client, active, deadline.capped(ACTIVATION_GRACE))
}

pub(crate) fn focus_window(
    bus: &SharedBus,
    win: &WindowInfo,
    lease: &InteractionLease,
) -> Result<(), AdapterError> {
    let deadline = lease.deadline();
    let world = focus_world::LiveWorld::new(bus, win, deadline)?;
    run(&world, win, deadline)
}

#[cfg(test)]
#[path = "focus_window_tests.rs"]
mod tests;
