use std::time::Instant;

use agent_desktop_core::{
    AdapterError, Deadline, ErrorCode, ObservationRequest, ObservationRoot, ObservationSource,
    ObservedTree, SnapshotSurface,
};

use super::projection::project;
use super::walker::walk_tree;
use super::{AtspiElement, ObservedRead, Walk, WalkLimits, atspi_element};
use crate::system::{SharedBus, window_id};

fn root_element(root: &ObservationRoot<'_>) -> Result<AtspiElement, AdapterError> {
    match root {
        ObservationRoot::Window(window) => {
            let (bus, path) = window_id::decode(&window.id)?;
            Ok(AtspiElement::new(bus, path, window.pid.get()))
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

fn require_window_surface(surface: SnapshotSurface) -> Result<(), AdapterError> {
    match surface {
        SnapshotSurface::Window => Ok(()),
        _ => Err(AdapterError::not_supported("observe_tree for this surface")),
    }
}

fn observe_with(
    root: ObservationRoot<'_>,
    request: &ObservationRequest,
    walker: impl FnOnce(
        &AtspiElement,
        &WalkLimits,
        Deadline,
    ) -> Result<Walk<ObservedRead>, AdapterError>,
) -> Result<ObservedTree, AdapterError> {
    let request = (*request).validate()?;
    require_window_surface(request.surface)?;
    let element = root_element(&root)?;
    let limits = WalkLimits::for_request(&request);
    let started = Instant::now();
    let walk =
        walker(&element, &limits, request.deadline).map_err(|error| window_gone(&root, error))?;
    let complete = walk.complete;
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
    observe_with(root, request, |element, limits, deadline| {
        walk_tree(bus, element, limits, deadline)
    })
}

#[cfg(test)]
#[path = "observe_tests.rs"]
mod tests;
