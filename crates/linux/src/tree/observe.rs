use std::time::{Duration, Instant};

use agent_desktop_core::{
    AdapterError, Deadline, ErrorCode, ObservationRequest, ObservationRoot, ObservationSource,
    ObservedTree, SnapshotSurface,
};

use super::ancestry;
use super::node_position::place_nodes;
use super::projection::project;
use super::settle::{Settle, Step, has_depth_cut, has_web_document};
use super::walker::walk_tree;
use super::{AtspiElement, ObservedRead, Walk, WalkLimits, atspi_element};
use crate::system::{SharedBus, proc_stat, process_facts, window_id};

pub(super) fn is_web_process(pid: u32) -> bool {
    process_facts::read(pid).is_some_and(|facts| facts.web_family.is_some())
}

/// A window of a stopped process cannot be read, so observing it is
/// `APP_UNRESPONSIVE` at once instead of a walk that waits out its budget.
fn refuse_stopped(
    window: &agent_desktop_core::WindowInfo,
    is_stopped: impl Fn(u32) -> bool,
) -> Result<(), AdapterError> {
    if is_stopped(window.pid.get()) {
        return Err(AdapterError::app_unresponsive(&window.app));
    }
    Ok(())
}

fn root_element(root: &ObservationRoot<'_>) -> Result<AtspiElement, AdapterError> {
    match root {
        ObservationRoot::Window(window) => {
            refuse_stopped(window, proc_stat::is_stopped)?;
            let (bus, path) = window_id::decode(&window.id)?;
            let mut element = AtspiElement::new(bus, path, window.pid.get());
            element.set_ancestry(ancestry::of_root(&element));
            Ok(element)
        }
        ObservationRoot::Element { handle, .. } => atspi_element(handle).cloned(),
    }
}

fn window_gone(root: &ObservationRoot<'_>, error: AdapterError) -> AdapterError {
    let is_window = matches!(root, ObservationRoot::Window(_));
    let vanished = matches!(
        error.code,
        ErrorCode::AppNotFound | ErrorCode::ElementNotFound
    );
    if is_window && vanished {
        return AdapterError::new(
            ErrorCode::WindowNotFound,
            "The window is gone or its application left the accessibility bus",
        )
        .with_suggestion("Run 'list-windows' to see the windows that exist now")
        .with_platform_detail(error.message);
    }
    error
}

fn require_surface(surface: SnapshotSurface) -> Result<(), AdapterError> {
    match surface {
        SnapshotSurface::Window | SnapshotSurface::Sheet => Ok(()),
        _ => Err(AdapterError::not_supported("observe_tree for this surface")),
    }
}

fn sheet_absent() -> AdapterError {
    AdapterError::new(
        ErrorCode::ElementNotFound,
        "The window does not present a sheet surface",
    )
    .with_suggestion("Open a modal dialog, or snapshot the window surface")
}

type Walker<'a> = dyn FnMut(&AtspiElement, &WalkLimits, Deadline) -> Result<Walk<ObservedRead>, AdapterError>
    + 'a;

/// Walks until the tree has stopped growing when the window of a web engine
/// is observed, otherwise once. An element a ref already names sits in a tree
/// that has filled, and a small subtree of it never exceeds the size of an
/// empty browser shell, so waiting for it to grow would spend the whole
/// budget. The second value is false when the wait ended on its budget with
/// the tree still changing. A later read that fails because the budget ran
/// out returns the walk before it; any other failure is the failure of the
/// observation.
fn settled_walk(
    root: &ObservationRoot<'_>,
    element: &AtspiElement,
    limits: &WalkLimits,
    request: &ObservationRequest,
    host: &mut SettleHost<'_>,
) -> Result<(Walk<ObservedRead>, bool), AdapterError> {
    let web_process = (host.is_web_process)(element.pid());
    let mut tracker: Option<(Settle, Instant)> = None;
    let mut previous: Option<Walk<ObservedRead>> = None;
    loop {
        let walk = match (host.walker)(element, limits, request.deadline) {
            Ok(walk) => walk,
            Err(error) if error.code == ErrorCode::Timeout && previous.is_some() => {
                return previous.map(|walk| (walk, false)).ok_or(error);
            }
            Err(error) => return Err(window_gone(root, error)),
        };
        if request.surface == SnapshotSurface::Sheet
            && matches!(root, ObservationRoot::Window(_))
            && !super::surfaces::is_sheet(&walk.root.payload.node)
        {
            return Err(sheet_absent());
        }
        let waits = matches!(root, ObservationRoot::Window(_))
            && (web_process || has_web_document(&walk.root));
        if !walk.complete || !waits {
            return Ok((walk, true));
        }
        let (settle, since) = tracker
            .get_or_insert_with(|| (Settle::new(request.deadline.remaining()), Instant::now()));
        match settle.after_walk(walk.node_count, since.elapsed(), has_depth_cut(&walk.root)) {
            Step::Settled => return Ok((walk, true)),
            Step::GiveUp => return Ok((walk, false)),
            Step::Wait(gap) => {
                (host.pause)(gap);
                previous = Some(walk);
            }
        }
    }
}

/// What the settle loop needs from outside the tree: the walk itself, whether
/// a process is a web engine, and how to wait.
struct SettleHost<'a> {
    walker: &'a mut Walker<'a>,
    is_web_process: &'a dyn Fn(u32) -> bool,
    pause: &'a mut dyn FnMut(Duration),
}

fn observe_with(
    root: ObservationRoot<'_>,
    request: &ObservationRequest,
    mut host: SettleHost<'_>,
) -> Result<ObservedTree, AdapterError> {
    let request = (*request).validate()?;
    require_surface(request.surface)?;
    let element = root_element(&root)?;
    let limits = WalkLimits {
        root_in_web: element.in_web_document(),
        ..WalkLimits::for_request(&request)
    };
    let started = Instant::now();
    let (walk, settled) = settled_walk(&root, &element, &limits, &request, &mut host)?;
    let complete = walk.complete && settled;
    let (subtree, mut stats) = project(walk, &limits);
    stats.reads.health.deadline_exhausted += u64::from(!complete && request.deadline.is_expired());
    stats.elapsed_us = u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);
    ObservedTree::from_roots(
        vec![subtree],
        ObservationSource::from_root(&root, request.surface),
        stats,
        complete,
    )
}

/// Observes the tree under a window or an element: the walk, then the
/// projection into core's observed tree. An observation the deadline or a
/// budget cut short is a success that reports `structurally_complete` false.
pub(crate) fn observe_tree(
    bus: &SharedBus,
    root: ObservationRoot<'_>,
    request: &ObservationRequest,
) -> Result<ObservedTree, AdapterError> {
    let mut walker = |element: &AtspiElement, limits: &WalkLimits, deadline: Deadline| {
        let mut walk = walk_tree(bus, element, limits, deadline)?;
        place_nodes(bus, &mut walk, deadline);
        Ok(walk)
    };
    let mut pause = std::thread::sleep;
    observe_with(
        root,
        request,
        SettleHost {
            walker: &mut walker,
            is_web_process: &is_web_process,
            pause: &mut pause,
        },
    )
}

#[cfg(test)]
#[path = "observe_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "observe_settle_tests.rs"]
mod settle_tests;
