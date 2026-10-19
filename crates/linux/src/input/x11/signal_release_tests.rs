use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc::channel};
use std::thread;
use std::time::{Duration, Instant};

use super::release_before;
use crate::input::x11::fake_sink::deadline;
use crate::input::x11::progress::Progress;
use crate::input::x11::worker_slot::WorkerSlot;

fn worker_that_releases_after_cancel(
    slot: &Arc<WorkerSlot>,
    progress: &Arc<Progress>,
    released: &Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    let claim = slot.claim(deadline(1_000)).expect("the worker's claim");
    let progress = Arc::clone(progress);
    let released = Arc::clone(released);
    thread::spawn(move || {
        while !progress.is_cancelled() {
            thread::sleep(Duration::from_millis(5));
        }
        thread::sleep(Duration::from_millis(100));
        released.store(true, Ordering::SeqCst);
        drop(claim);
    })
}

#[test]
fn the_signal_waits_for_the_worker_to_release_before_the_default_action_runs() {
    let slot = Arc::new(WorkerSlot::default());
    let progress = Arc::new(Progress::default());
    let released = Arc::new(AtomicBool::new(false));
    let worker = worker_that_releases_after_cancel(&slot, &progress, &released);
    let (seen, observed) = channel();

    release_before(&progress, &slot, Duration::from_secs(5), || {
        seen.send(released.load(Ordering::SeqCst)).expect("send");
    });

    assert!(observed.recv().expect("the follow-up ran"));
    worker.join().expect("the worker ends");
}

#[test]
fn a_worker_that_never_finishes_holds_the_signal_back_only_for_the_wait() {
    let slot = Arc::new(WorkerSlot::default());
    let progress = Arc::new(Progress::default());
    let _stuck = slot
        .claim(deadline(1_000))
        .expect("a claim that is never freed");
    let started = Instant::now();

    release_before(&progress, &slot, Duration::from_millis(200), || {});

    assert!(progress.is_cancelled());
    assert!(started.elapsed() < Duration::from_secs(2));
}
