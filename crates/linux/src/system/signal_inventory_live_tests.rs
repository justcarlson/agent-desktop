use std::process::Command;
use std::time::{Duration, Instant};

use agent_desktop_core::{Deadline, ErrorCode, WindowFilter, WindowInfo};

use super::{RootRead, run_pass};
use crate::system::registry::{read_registry, read_registry_for};
use crate::system::registry_frames::read_frames;
use crate::system::{SharedBus, list_windows, resolve_window_strict};
use crate::tree::live_launch::Launched;

const SETTLE: Duration = Duration::from_secs(40);

fn deadline() -> Deadline {
    Deadline::after(10_000).expect("a deadline builds")
}

fn everything() -> WindowFilter {
    WindowFilter {
        focused_only: false,
        app: None,
    }
}

fn window_of(bus: &SharedBus, pid: u32) -> WindowInfo {
    let started = Instant::now();
    while started.elapsed() < SETTLE {
        let listed = list_windows(bus, &everything(), deadline()).unwrap_or_default();
        if let Some(window) = listed.into_iter().find(|w| w.pid.get() == pid) {
            return window;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    panic!("pid {pid} never listed a window");
}

fn signal(name: &str, pid: u32) {
    let _ = Command::new("kill").args([name, &pid.to_string()]).status();
}

struct Stopped(u32);

impl Drop for Stopped {
    fn drop(&mut self) {
        signal("-CONT", self.0);
    }
}

fn min_median_max(mut samples: Vec<u128>) -> (u128, u128, u128) {
    samples.sort_unstable();
    (
        samples[0],
        samples[samples.len() / 2],
        samples[samples.len() - 1],
    )
}

fn timed<T>(mut run: impl FnMut() -> T) -> (T, u128) {
    let started = Instant::now();
    let value = run();
    (value, started.elapsed().as_millis())
}

#[test]
#[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]
fn signal_inventory_live_a_stopped_target_is_skipped_slow_roots_read_timed_out_and_resolution_names_it()
 {
    let bus = SharedBus::default();
    let mut launched = Launched::new();
    let pid = launched.spawn("gnome-calculator", &[]);
    let window = window_of(&bus, pid);

    let mut full = Vec::new();
    let mut scoped = Vec::new();
    for _ in 0..7 {
        let ((registry, pass), ms) = timed(|| {
            let registry = read_registry(&bus, deadline()).expect("registry");
            let pass = run_pass(&bus, &registry, |_| true, deadline()).expect("pass");
            (registry, pass)
        });
        full.push(ms);
        assert!(pass.outcomes.iter().all(|o| o.read == RootRead::Read));
        assert!(pass.windows.iter().any(|w| w.info.pid.get() == pid));
        drop(registry);
        let (_, ms) = timed(|| {
            let registry = read_registry_for(&bus, Some(pid), deadline()).expect("registry");
            run_pass(&bus, &registry, |app| app.root.pid() == pid, deadline()).expect("pass")
        });
        scoped.push(ms);
    }
    println!(
        "signal_inventory_live alive: registry+pass all apps (min,median,max ms) = {:?}; process-scoped = {:?}",
        min_median_max(full),
        min_median_max(scoped)
    );

    signal("-STOP", pid);
    let stop = Stopped(pid);
    let mut stopped_reads = Vec::new();
    for _ in 0..7 {
        let (_, ms) = timed(|| read_registry(&bus, deadline()).expect("registry"));
        stopped_reads.push(ms);
    }
    println!(
        "signal_inventory_live registry read with one stopped app (min,median,max ms) = {:?}",
        min_median_max(stopped_reads)
    );
    let registry = read_registry(&bus, deadline()).expect("registry with a stopped app");
    let app = registry
        .apps
        .iter()
        .find(|app| app.root.pid() == pid)
        .expect("a stopped app stays on the registry");
    assert!(!app.answered, "a stopped app does not answer its root");

    let (pass, ms) = timed(|| run_pass(&bus, &registry, |_| true, deadline()).expect("pass"));
    assert!(
        pass.windows
            .iter()
            .any(|w| w.info.pid.get() == pid && !w.info.state.accessible),
        "the stopped app's window is recovered from the display server"
    );
    assert!(
        pass.outcomes
            .iter()
            .all(|o| registry.apps[o.app].root.pid() != pid)
    );
    println!(
        "signal_inventory_live stopped target, unscoped pass: {ms}ms, {} outcomes",
        pass.outcomes.len()
    );

    let (only, ms) =
        timed(|| run_pass(&bus, &registry, |a| a.root.pid() == pid, deadline()).expect("pass"));
    assert!(only.outcomes.is_empty());
    assert!(
        only.windows
            .iter()
            .all(|w| w.info.pid.get() == pid && !w.info.state.accessible),
        "a scoped pass on a stopped app lists its display-server windows"
    );
    println!("signal_inventory_live stopped target, scoped pass: {ms}ms");

    let roots = vec![app.root.clone()];
    let (frames, ms) = timed(|| read_frames(&bus, &roots, deadline()).expect("frames"));
    assert_eq!(frames[0].read, RootRead::TimedOut);
    assert!(frames[0].nodes.is_empty());
    println!("signal_inventory_live slow (unanswering) root read directly: TimedOut after {ms}ms");

    let (error, ms) =
        timed(|| resolve_window_strict(&bus, &window, deadline()).expect_err("stopped"));
    assert_eq!(error.code, ErrorCode::AppUnresponsive, "{}", error.message);
    println!("signal_inventory_live stopped Calculator resolves to APP_UNRESPONSIVE in {ms}ms");

    drop(stop);
    let started = Instant::now();
    loop {
        if resolve_window_strict(&bus, &window, deadline()).is_ok() {
            break;
        }
        assert!(
            started.elapsed() < SETTLE,
            "the window did not resolve after SIGCONT"
        );
        std::thread::sleep(Duration::from_millis(200));
    }

    signal("-KILL", pid);
    let started = Instant::now();
    loop {
        match resolve_window_strict(&bus, &window, deadline()) {
            Err(error) if error.code == ErrorCode::WindowNotFound => break,
            _ => {
                assert!(
                    started.elapsed() < SETTLE,
                    "a killed app did not read WINDOW_NOT_FOUND"
                );
                std::thread::sleep(Duration::from_millis(200));
            }
        }
    }
}
