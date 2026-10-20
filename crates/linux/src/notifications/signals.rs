use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use agent_desktop_core::{AdapterError, Deadline, ErrorCode};
use futures_lite::future::or;
use futures_util::StreamExt;
use zbus::message::Type;
use zbus::{MatchRule, MessageStream};

use crate::system::session_bus::SessionBus;

pub(crate) const SETTLE_BUDGET: Duration = Duration::from_millis(3_000);
pub(crate) const QUIET_AFTER_SIGNAL: Duration = Duration::from_millis(150);
const READY_WAIT: Duration = Duration::from_millis(2_000);
const STOP_POLL: Duration = Duration::from_millis(50);
const NOTIFICATIONS_INTERFACE: &str = "org.freedesktop.Notifications";
const NOTIFICATIONS_PATH: &str = "/org/freedesktop/Notifications";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Signal {
    ActionInvoked(u32),
    Closed(u32),
}

/// A subscription made before a click. Dropping it or collecting from it
/// removes the match rule and ends its reader thread.
pub(crate) trait Subscription {
    fn collect(self: Box<Self>, budget: Duration) -> Vec<Signal>;
}

pub(crate) trait SignalSource {
    fn subscribe(&self, deadline: Deadline) -> Result<Box<dyn Subscription>, AdapterError>;
}

/// A reader thread feeding a channel. The thread owns no state the caller
/// needs after the call: `stop` ends it and the join waits for it.
pub(crate) struct Reader {
    stop: Arc<AtomicBool>,
    inbox: Receiver<Signal>,
    thread: Option<JoinHandle<()>>,
}

impl Reader {
    pub(crate) fn spawn(
        run: impl FnOnce(Arc<AtomicBool>, Sender<Signal>) + Send + 'static,
    ) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let (outbox, inbox) = channel();
        let flag = Arc::clone(&stop);
        let thread = std::thread::spawn(move || run(flag, outbox));
        Self {
            stop,
            inbox,
            thread: Some(thread),
        }
    }

    fn halt(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Subscription for Reader {
    fn collect(mut self: Box<Self>, budget: Duration) -> Vec<Signal> {
        let signals = gather(&self.inbox, budget, QUIET_AFTER_SIGNAL);
        self.halt();
        signals
    }
}

impl Drop for Reader {
    fn drop(&mut self) {
        self.halt();
    }
}

/// Reads the channel until the budget ends, the reader ends, or the channel
/// has been quiet for `quiet` after at least one signal arrived. The budget
/// bounds the wait; the operation deadline never does.
pub(crate) fn gather(inbox: &Receiver<Signal>, budget: Duration, quiet: Duration) -> Vec<Signal> {
    let started = Instant::now();
    let mut seen = Vec::new();
    loop {
        let left = budget.saturating_sub(started.elapsed());
        if left.is_zero() {
            return seen;
        }
        let wait = if seen.is_empty() {
            left
        } else {
            left.min(quiet)
        };
        match inbox.recv_timeout(wait) {
            Ok(signal) => seen.push(signal),
            Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => return seen,
        }
    }
}

pub(crate) struct BusSignals;

impl SignalSource for BusSignals {
    fn subscribe(&self, deadline: Deadline) -> Result<Box<dyn Subscription>, AdapterError> {
        let connection = SessionBus::get(deadline)?;
        let (ready_tx, ready_rx) = channel::<Result<(), String>>();
        let reader = Reader::spawn(move |stop, outbox| {
            async_io::block_on(listen(connection, stop, outbox, ready_tx, deadline));
        });
        match ready_rx.recv_timeout(READY_WAIT.min(deadline.remaining())) {
            Ok(Ok(())) => Ok(Box::new(reader)),
            Ok(Err(detail)) => Err(subscribe_failed(detail)),
            Err(_) => {
                drop(reader);
                if deadline.is_expired() {
                    Err(deadline.timeout_error())
                } else {
                    Err(subscribe_failed(
                        "the match rule was not confirmed".to_owned(),
                    ))
                }
            }
        }
    }
}

fn subscribe_failed(detail: String) -> AdapterError {
    AdapterError::new(
        ErrorCode::Internal,
        "The notification signals could not be subscribed to",
    )
    .with_platform_detail(detail)
}

async fn listen(
    connection: zbus::Connection,
    stop: Arc<AtomicBool>,
    outbox: Sender<Signal>,
    ready: Sender<Result<(), String>>,
    deadline: Deadline,
) {
    let stream = match await_startup(open_stream(&connection), &stop, deadline).await {
        Ok(stream) => stream,
        Err(StartupEnd::Cancelled) => {
            let _ = ready.send(Err("subscription startup was cancelled".to_owned()));
            return;
        }
        Err(StartupEnd::Deadline) => {
            let _ = ready.send(Err(
                "the operation deadline expired during subscription startup".to_owned(),
            ));
            return;
        }
        Err(StartupEnd::Failed(error)) => {
            let _ = ready.send(Err(error.to_string()));
            return;
        }
    };
    let mut stream = stream;
    let _ = ready.send(Ok(()));
    while !stop.load(Ordering::SeqCst) {
        let next = or(async { Some(stream.next().await) }, async {
            async_io::Timer::after(STOP_POLL).await;
            None
        })
        .await;
        match next {
            Some(Some(Ok(message))) => {
                if let Some(signal) = parse(&message) {
                    let _ = outbox.send(signal);
                }
            }
            Some(_) => return,
            None => {}
        }
    }
}

enum StartupEnd {
    Cancelled,
    Deadline,
    Failed(zbus::Error),
}

async fn await_startup<T>(
    future: impl std::future::Future<Output = zbus::Result<T>>,
    stop: &AtomicBool,
    deadline: Deadline,
) -> Result<T, StartupEnd> {
    futures_lite::future::or(async { future.await.map_err(StartupEnd::Failed) }, async {
        loop {
            if stop.load(Ordering::SeqCst) {
                return Err(StartupEnd::Cancelled);
            }
            let remaining = deadline.remaining();
            if remaining.is_zero() {
                return Err(StartupEnd::Deadline);
            }
            async_io::Timer::after(STOP_POLL.min(remaining)).await;
        }
    })
    .await
}

async fn open_stream(connection: &zbus::Connection) -> zbus::Result<MessageStream> {
    let rule = MatchRule::builder()
        .msg_type(Type::Signal)
        .interface(NOTIFICATIONS_INTERFACE)?
        .path(NOTIFICATIONS_PATH)?
        .build();
    MessageStream::for_match_rule(rule, connection, None).await
}

fn parse(message: &zbus::Message) -> Option<Signal> {
    let header = message.header();
    let member = header.member()?.as_str();
    match member {
        "ActionInvoked" => {
            let (id, _key): (u32, String) = message.body().deserialize().ok()?;
            Some(Signal::ActionInvoked(id))
        }
        "NotificationClosed" => {
            let (id, _reason): (u32, u32) = message.body().deserialize().ok()?;
            Some(Signal::Closed(id))
        }
        _ => None,
    }
}

#[cfg(test)]
#[path = "signals_tests.rs"]
mod tests;
