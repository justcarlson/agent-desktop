use crate::system::launch::launch_sample::Sample;
use crate::system::launch::launch_step::Step;
use std::collections::HashSet;
use std::os::unix::process::ExitStatusExt;
use std::process::{Child, ExitStatus};
use std::time::{Duration, Instant};

use agent_desktop_core::{AdapterError, Deadline, DeliverySemantics, ErrorCode, ProcessId};

use super::launch_attach::matches;
use crate::system::SharedBus;
use crate::system::process::running_app::Running;
use crate::system::registry::{Registry, read_registry};

pub(in crate::system) const POLL_INTERVAL: Duration = Duration::from_millis(50);
const POLL_LIMIT: Duration = Duration::from_millis(250);
const EXIT_GRACE: Duration = Duration::from_millis(1_000);

fn pick(mut found: Vec<&Running>) -> Option<Step> {
    match found.len() {
        0 => None,
        1 => Some(Step::Owner(found.remove(0).clone())),
        _ => Some(Step::Ambiguous(found.iter().map(|app| app.pid).collect())),
    }
}

/// The next move of a launch. The owner is the child while it runs and shows
/// on the registry, else an application matching the executable identity that was
/// absent before the spawn, else (when the launcher exited cleanly after
/// forwarding) the one that was already running. A launcher killed by a signal
/// or exiting with a failure and no owner is a failed launch, and a clean
/// exit with no owner is one too once the registry had its grace.
pub(crate) fn step(sample: &Sample<'_>) -> Step {
    let candidates: Vec<&Running> = sample
        .apps
        .iter()
        .filter(|app| matches(sample.identity, app))
        .collect();
    let new: Vec<&Running> = candidates
        .iter()
        .copied()
        .filter(|app| !sample.before.contains(&app.pid))
        .collect();
    let own = sample.apps.iter().find(|app| app.pid == sample.child_pid);
    match sample.child {
        None => own
            .map(|app| Step::Owner(app.clone()))
            .or_else(|| pick(new))
            .unwrap_or(Step::Wait),
        Some(status) if status.code().is_none() => Step::Failed(status),
        Some(status) => {
            let code = status.code();
            pick(new)
                .or_else(|| {
                    if code == Some(0) {
                        pick(candidates)
                    } else {
                        None
                    }
                })
                .unwrap_or(if code.is_some_and(|code| code != 0) || sample.settled {
                    Step::Failed(status)
                } else {
                    Step::Wait
                })
        }
    }
}

/// The registry as one record per process, with the facts launching reads.
pub(crate) fn running_of(registry: &Registry) -> Vec<Running> {
    let mut seen = HashSet::new();
    registry
        .apps
        .iter()
        .filter(|app| seen.insert(app.root.pid()))
        .map(|app| Running {
            executable: app.facts.executable.clone(),
            pid: app.root.pid(),
            instance: app.facts.instance.clone(),
            name: app.name.clone(),
        })
        .collect()
}

/// A spawned process that is reaped when this guard goes, so an application
/// that outlives a long-running host never becomes a zombie.
pub(crate) struct Spawned {
    child: Option<Child>,
    pub(crate) pid: u32,
}

impl Spawned {
    pub(crate) fn new(child: Child) -> Self {
        Self {
            pid: child.id(),
            child: Some(child),
        }
    }

    fn state(&mut self) -> Option<ExitStatus> {
        self.child
            .as_mut()
            .and_then(|child| child.try_wait().ok().flatten())
    }
}

impl Drop for Spawned {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take()
            && matches!(child.try_wait(), Ok(None))
        {
            std::thread::spawn(move || {
                let _ = child.wait();
            });
        }
    }
}

fn failed(status: ExitStatus, identifier: &str) -> AdapterError {
    let (what, detail) = match status.code() {
        Some(code) => (
            format!(
                "'{identifier}' exited with status {code} without a verified application owner"
            ),
            serde_json::json!({ "kind": "exited", "status": code }),
        ),
        None => {
            let signal = status.signal().unwrap_or_default();
            (
                format!("'{identifier}' was killed by signal {signal} before it started"),
                serde_json::json!({ "kind": "crashed", "signal": signal }),
            )
        }
    };
    AdapterError::new(ErrorCode::ActionFailed, what)
        .with_details(detail)
        .with_suggestion(
            "The launcher may have run and handed off to an application this tool cannot match; run 'list-apps' to see what is running.",
        )
        .with_disposition(DeliverySemantics::delivered_unverified())
}

pub(in crate::system) fn pids_of(pids: &[u32]) -> Vec<ProcessId> {
    pids.iter().copied().map(ProcessId::new).collect()
}

fn ambiguous(pids: &[u32], identifier: &str) -> AdapterError {
    AdapterError::ambiguous_process_target(
        format!("Several new applications match '{identifier}'"),
        &pids_of(pids),
    )
    .with_disposition(DeliverySemantics::delivered_unverified())
}

fn after_spawn(error: AdapterError) -> AdapterError {
    error.with_disposition(DeliverySemantics::delivered_unverified())
}

/// Polls the registry until the launch has an owner, the launcher failed, or
/// the deadline. A child that is alive with no registry entry at the deadline
/// is reported as `NotOnBus`: the pid is the spawned process's.
pub(crate) fn find_owner(
    bus: &SharedBus,
    spawned: &mut Spawned,
    identity: &Option<crate::system::process::executable_identity::ExecutableIdentity>,
    before: &HashSet<u32>,
    deadline: Deadline,
) -> Result<Option<Running>, AdapterError> {
    let identifier = "the application";
    let mut interval = POLL_INTERVAL;
    let mut exited_at: Option<Instant> = None;
    loop {
        let child = spawned.state();
        if child.is_some() {
            exited_at.get_or_insert_with(Instant::now);
        }
        let expired = deadline.is_expired();
        let registry = match read_registry(bus, deadline) {
            Ok(registry) => Some(registry),
            Err(error) if error.code == ErrorCode::Timeout => None,
            Err(error) => return Err(after_spawn(error)),
        };
        let apps = registry.as_ref().map(running_of).unwrap_or_default();
        let settled = expired || exited_at.is_some_and(|at| at.elapsed() >= EXIT_GRACE);
        let sample = Sample {
            child_pid: spawned.pid,
            child,
            apps: &apps,
            before,
            identity,
            settled,
        };
        match step(&sample) {
            Step::Owner(owner) => return Ok(Some(owner)),
            Step::Ambiguous(pids) => return Err(ambiguous(&pids, identifier)),
            Step::Failed(status) => return Err(failed(status, identifier)),
            Step::Wait if expired => return Ok(None),
            Step::Wait => {}
        }
        std::thread::sleep(interval.min(deadline.remaining()));
        interval = (interval * 3 / 2).min(POLL_LIMIT);
    }
}

#[cfg(test)]
#[path = "launch_observe_tests.rs"]
mod tests;
