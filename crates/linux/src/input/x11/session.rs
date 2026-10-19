use std::sync::Arc;
use std::sync::mpsc::{RecvTimeoutError, channel};
use std::thread;
use std::time::Duration;

use agent_desktop_core::{AdapterError, Deadline, ErrorCode};

use super::held_journal::HeldJournal;
use super::progress::Progress;
use super::signal_release;
use super::sink::X11Sink;
use super::sink_error::SinkError;
use super::worker::{self, ABORT_SUGGESTION, Link, Report};
use super::worker_slot::{self, WorkerSlot};
use super::xtest_sink::XtestSink;
use crate::input::delivery::{Delivery, Gate};
use crate::input::keymap::Keymap;
use crate::input::plan::Plan;
use crate::input::release_state::ReleaseState;

const WORKER_GRACE: Duration = Duration::from_millis(250);
const RELEASE_ALLOWANCE: Duration = Duration::from_secs(2);

pub(crate) struct CallEnv {
    pub(crate) deadline: Deadline,
    pub(crate) journal: Option<HeldJournal>,
    pub(crate) slot: Arc<WorkerSlot>,
    pub(crate) release_on_signal: bool,
}

pub(crate) fn run(
    display: &str,
    plans: Vec<Plan>,
    gate: Gate<'_>,
    deadline: Deadline,
) -> Result<Delivery, AdapterError> {
    let display = display.to_owned();
    let journal = HeldJournal::in_runtime_dir(&display);
    let cutoff = deadline.remaining() + RELEASE_ALLOWANCE;
    run_journaled(
        move || XtestSink::connect(&display, deadline, cutoff),
        plans,
        gate,
        CallEnv {
            deadline,
            journal,
            slot: worker_slot::shared(),
            release_on_signal: true,
        },
    )
}

pub(crate) fn live_keymap(display: &str, deadline: Deadline) -> Result<Keymap, AdapterError> {
    let display = display.to_owned();
    let cutoff = deadline.remaining() + RELEASE_ALLOWANCE;
    let read = crate::system::x11::start(move || {
        let mut sink = XtestSink::connect(&display, deadline, cutoff)?;
        sink.keymap().map_err(SinkError::into_adapter)
    });
    read.finish(deadline)
        .unwrap_or_else(|| Err(worker::mark_started(deadline.timeout_error(), 0)))
}

#[cfg(test)]
pub(crate) fn run_with<S: X11Sink + 'static>(
    open: impl FnOnce() -> Result<S, AdapterError> + Send + 'static,
    plans: Vec<Plan>,
    gate: Gate<'_>,
    deadline: Deadline,
) -> Result<Delivery, AdapterError> {
    run_journaled(
        open,
        plans,
        gate,
        CallEnv {
            deadline,
            journal: None,
            slot: Arc::new(WorkerSlot::default()),
            release_on_signal: false,
        },
    )
}

#[cfg(test)]
pub(crate) fn run_with_slot<S: X11Sink + 'static>(
    open: impl FnOnce() -> Result<S, AdapterError> + Send + 'static,
    plans: Vec<Plan>,
    gate: Gate<'_>,
    (deadline, slot): (Deadline, Arc<WorkerSlot>),
) -> Result<Delivery, AdapterError> {
    run_journaled(
        open,
        plans,
        gate,
        CallEnv {
            deadline,
            journal: None,
            slot,
            release_on_signal: false,
        },
    )
}

pub(crate) fn run_journaled<S: X11Sink + 'static>(
    open: impl FnOnce() -> Result<S, AdapterError> + Send + 'static,
    plans: Vec<Plan>,
    gate: Gate<'_>,
    env: CallEnv,
) -> Result<Delivery, AdapterError> {
    let CallEnv {
        deadline,
        journal,
        slot,
        release_on_signal,
    } = env;
    let claim = slot.claim(deadline)?;
    let progress = Arc::new(Progress::default());
    let _armed =
        release_on_signal.then(|| signal_release::arm(Arc::clone(&progress), Arc::clone(&slot)));
    let (reports, inbox) = channel();
    let (proceed_tx, proceed) = channel();
    let link = Link {
        reports,
        proceed,
        journal,
    };
    let worker_progress = Arc::clone(&progress);
    let spawned = thread::Builder::new()
        .name("x11-input".to_owned())
        .spawn(move || {
            let _claim = claim;
            match open() {
                Ok(sink) => worker::work(sink, plans, &link, deadline, &worker_progress),
                Err(error) => {
                    let _ = link.reports.send(Report::Done(Err(error)));
                }
            }
        });
    if spawned.is_err() {
        return Err(AdapterError::internal("The input worker could not start")
            .with_disposition(agent_desktop_core::DeliverySemantics::not_delivered()));
    }
    loop {
        match inbox.recv_timeout(deadline.remaining() + WORKER_GRACE) {
            Ok(Report::Gate) => match gate() {
                Ok(()) => {
                    let _ = proceed_tx.send(true);
                }
                Err(error) => {
                    let _ = proceed_tx.send(false);
                    progress.cancel();
                    return Err(ReleaseState::resumed(progress.delivered(), false)
                        .enrich_error(error, ABORT_SUGGESTION));
                }
            },
            Ok(Report::Done(outcome)) => return outcome,
            Err(RecvTimeoutError::Timeout) => {
                progress.cancel();
                return Err(abandoned(deadline, &progress));
            }
            Err(RecvTimeoutError::Disconnected) => {
                let lost = AdapterError::new(
                    ErrorCode::Internal,
                    "The input worker ended without an answer",
                );
                return Err(ReleaseState::resumed(progress.delivered(), true)
                    .enrich_error(lost, ABORT_SUGGESTION));
            }
        }
    }
}

fn abandoned(deadline: Deadline, progress: &Progress) -> AdapterError {
    let delivered = progress.delivered();
    let error = worker::mark_started(deadline.timeout_error(), delivered);
    ReleaseState::resumed(delivered, delivered > 0).enrich_error(error, ABORT_SUGGESTION)
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
