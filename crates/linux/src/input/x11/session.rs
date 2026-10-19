use std::sync::Arc;
use std::sync::mpsc::{RecvTimeoutError, channel};
use std::thread;
use std::time::Duration;

use agent_desktop_core::{AdapterError, Deadline, ErrorCode};

use super::held_journal::HeldJournal;
use super::progress::Progress;
use super::sink::X11Sink;
use super::sink_error::SinkError;
use super::worker::{self, Link, Report};
use super::xtest_sink::XtestSink;
use crate::input::delivery::Delivery;
use crate::input::keymap::Keymap;
use crate::input::plan::Plan;
use crate::input::release_state::ReleaseState;

const WORKER_GRACE: Duration = Duration::from_millis(250);
const ABORT_SUGGESTION: &str =
    "Run 'snapshot' to read the real state before retrying: some input was already delivered";

pub(crate) fn run(
    display: &str,
    plans: Vec<Plan>,
    gate: &mut dyn FnMut() -> Result<(), AdapterError>,
    deadline: Deadline,
) -> Result<Delivery, AdapterError> {
    let display = display.to_owned();
    let journal = HeldJournal::in_runtime_dir(&display);
    run_journaled(
        move || XtestSink::connect(&display),
        plans,
        gate,
        (deadline, journal),
    )
}

pub(crate) fn live_keymap(display: &str, deadline: Deadline) -> Result<Keymap, AdapterError> {
    let display = display.to_owned();
    let read = crate::system::x11::start(move || {
        let mut sink = XtestSink::connect(&display)?;
        sink.keymap().map_err(SinkError::into_adapter)
    });
    read.finish(deadline)
        .unwrap_or_else(|| Err(worker::mark_started(deadline.timeout_error(), 0)))
}

#[cfg(test)]
pub(crate) fn run_with<S: X11Sink + 'static>(
    open: impl FnOnce() -> Result<S, AdapterError> + Send + 'static,
    plans: Vec<Plan>,
    gate: &mut dyn FnMut() -> Result<(), AdapterError>,
    deadline: Deadline,
) -> Result<Delivery, AdapterError> {
    run_journaled(open, plans, gate, (deadline, None))
}

pub(crate) fn run_journaled<S: X11Sink + 'static>(
    open: impl FnOnce() -> Result<S, AdapterError> + Send + 'static,
    plans: Vec<Plan>,
    gate: &mut dyn FnMut() -> Result<(), AdapterError>,
    (deadline, journal): (Deadline, Option<HeldJournal>),
) -> Result<Delivery, AdapterError> {
    let progress = Arc::new(Progress::default());
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
        .spawn(move || match open() {
            Ok(sink) => worker::work(sink, plans, &link, deadline, &worker_progress),
            Err(error) => {
                let _ = link.reports.send(Report::Done(Err(error)));
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
