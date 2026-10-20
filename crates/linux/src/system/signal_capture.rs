use agent_desktop_core::{
    AdapterError, Deadline, SignalBaseline, SignalCompleteness, SignalFilter, WindowInfo,
};
use serde_json::json;

use super::SharedBus;
use super::bus::ensure_budget;
use super::focus::focus_is_indeterminate;
use super::listed_window::ListedWindow;
use super::nested_scan::{NestedScan, bus_read, scan_windows};
use super::proc_stat;
use super::registry::{Registry, read_registry_for};
use super::registry_app::RegistryApp;
use super::registry_frames::RootRead;
use super::signal_boundary::{Scope, narrow};
use super::signal_inventory::{Pass, run_pass, skipped_selected};
use super::signal_population::apps_of;
use super::signal_surfaces::{nested_signals, sheet_signals};
use super::unresponsive::first_stopped;

trait CaptureWorld {
    fn registry(&self, pid: Option<u32>, deadline: Deadline) -> Result<Registry, AdapterError>;
    fn pass(
        &self,
        registry: &Registry,
        select: &dyn Fn(&RegistryApp) -> bool,
        deadline: Deadline,
    ) -> Result<Pass, AdapterError>;
    fn is_stopped(&self, pid: u32) -> bool;
    fn nested(&self, windows: &[ListedWindow], deadline: Deadline) -> NestedScan;
}

struct BusWorld<'a>(&'a SharedBus);

impl CaptureWorld for BusWorld<'_> {
    fn registry(&self, pid: Option<u32>, deadline: Deadline) -> Result<Registry, AdapterError> {
        read_registry_for(self.0, pid, deadline)
    }

    fn pass(
        &self,
        registry: &Registry,
        select: &dyn Fn(&RegistryApp) -> bool,
        deadline: Deadline,
    ) -> Result<Pass, AdapterError> {
        run_pass(self.0, registry, select, deadline)
    }

    fn is_stopped(&self, pid: u32) -> bool {
        proc_stat::is_stopped(pid)
    }

    fn nested(&self, windows: &[ListedWindow], deadline: Deadline) -> NestedScan {
        scan_windows(windows, bus_read(self.0, deadline))
    }
}

fn scope_of(filter: &SignalFilter) -> Scope {
    if filter.app.is_some() || filter.process.is_some() {
        Scope::Scoped
    } else {
        Scope::Unscoped
    }
}

fn selector(filter: &SignalFilter) -> impl Fn(&RegistryApp) -> bool + '_ {
    move |app| {
        filter
            .process
            .as_ref()
            .is_none_or(|process| app.root.pid() == process.pid.get())
            && filter.app.as_deref().is_none_or(|query| app.matches(query))
    }
}

fn stopped_scoped_target(
    world: &impl CaptureWorld,
    registry: &Registry,
    select: &dyn Fn(&RegistryApp) -> bool,
) -> Option<AdapterError> {
    let app = first_stopped(registry, select, |pid| world.is_stopped(pid))?;
    Some(
        AdapterError::app_unresponsive(&app.name).with_details(json!({
            "app": app.name,
            "pid": app.root.pid(),
            "process_instance": app.facts.instance,
        })),
    )
}

fn windows_of_pass(pass: &Pass) -> Vec<WindowInfo> {
    pass.windows
        .iter()
        .enumerate()
        .map(|(index, window)| {
            let mut info = window.info.clone();
            info.state.is_focused = pass.focused == Some(index);
            info
        })
        .collect()
}

fn assemble(
    registry: &Registry,
    pass: &Pass,
    scope: Scope,
    nested: &NestedScan,
    unread: bool,
) -> SignalBaseline {
    let read = pass
        .outcomes
        .iter()
        .all(|outcome| outcome.read == RootRead::Read)
        && !unread;
    let scoped = scope == Scope::Scoped;
    let surfaces = if scoped {
        let mut surfaces = sheet_signals(&pass.windows, &registry.apps);
        surfaces.extend(nested_signals(nested, &pass.windows, &registry.apps));
        surfaces
    } else {
        Vec::new()
    };
    SignalBaseline {
        windows: windows_of_pass(pass),
        apps: apps_of(&pass.windows, &registry.apps),
        surfaces,
        completeness: SignalCompleteness {
            windows: read && !focus_is_indeterminate(&pass.candidates, pass.focused),
            apps: read,
            surfaces: scoped && read && nested.complete,
        },
    }
}

fn capture(
    world: &impl CaptureWorld,
    filter: &SignalFilter,
    deadline: Deadline,
) -> Result<SignalBaseline, AdapterError> {
    ensure_budget(deadline)?;
    let scope = scope_of(filter);
    let registry = world.registry(filter.process.as_ref().map(|p| p.pid.get()), deadline)?;
    let select = selector(filter);
    if scope == Scope::Scoped
        && let Some(error) = stopped_scoped_target(world, &registry, &select)
    {
        return Err(error);
    }
    let pass = world.pass(&registry, &select, deadline)?;
    let nested = match scope {
        Scope::Scoped => world.nested(&pass.windows, deadline),
        Scope::Unscoped => NestedScan::complete_and_empty(),
    };
    let unread = skipped_selected(&registry, &pass, &select);
    let baseline = assemble(&registry, &pass, scope, &nested, unread);
    ensure_budget(deadline)?;
    Ok(baseline)
}

/// What the desktop looks like now, for core's `wait --event` to diff. The
/// filter selects apps before any frame is read. Every error that leaves is
/// `TIMEOUT`, `APP_UNRESPONSIVE` for a stopped scoped target, or the fatal
/// missing-bus error.
pub(crate) fn capture_signal_baseline(
    bus: &SharedBus,
    filter: &SignalFilter,
    deadline: Deadline,
) -> Result<SignalBaseline, AdapterError> {
    capture(&BusWorld(bus), filter, deadline)
        .map_err(|error| narrow(error, scope_of(filter), bus, deadline))
}

#[cfg(test)]
#[path = "signal_capture_tests.rs"]
mod tests;
