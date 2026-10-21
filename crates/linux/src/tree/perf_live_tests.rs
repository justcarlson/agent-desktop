use std::time::Instant;

use agent_desktop_core::commands::find::{FindArgs, FindFilterArgs, FindSelectionArgs};
use agent_desktop_core::{
    CommandContext, ErrorCode, ObservationOps, RefEntry, SnapshotSurface, WindowFilter,
};

use super::live_apps::each_app;
use super::live_rig::{deadline, rig, search_entry};
use super::resolve::Source;
use super::resolve_source::BusSource;
use super::walker::walk_tree;
use super::{AtspiElement, WalkLimits};
use crate::LinuxAdapter;
use crate::system::{SharedBus, resolve_window_strict, window_id};

const RUNS: usize = 7;

struct Timing {
    min: f64,
    median: f64,
    max: f64,
}

fn measure(mut run: impl FnMut()) -> Timing {
    let mut millis: Vec<f64> = (0..=RUNS)
        .map(|_| {
            let started = Instant::now();
            run();
            started.elapsed().as_secs_f64() * 1000.0
        })
        .skip(1)
        .collect();
    millis.sort_by(f64::total_cmp);
    Timing {
        min: millis[0],
        median: millis[RUNS / 2],
        max: millis[RUNS - 1],
    }
}

fn report(app: &str, what: &str, timing: &Timing) {
    println!(
        "perf {app} {what}: min/median/max ms = {:.1}/{:.1}/{:.1}",
        timing.min, timing.median, timing.max
    );
}

fn find_all(adapter: &LinuxAdapter, window: &str, role: &str) -> Result<(), String> {
    let args = FindArgs {
        app: None,
        window_id: Some(window.to_owned()),
        root: None,
        surface: SnapshotSurface::Window,
        filter: FindFilterArgs {
            role: Some(role.to_owned()),
            name: None,
            description: None,
            native_id: None,
            value: None,
            text: None,
            exact: false,
        },
        states: Vec::new(),
        selection: FindSelectionArgs {
            count: false,
            first: false,
            last: false,
            nth: None,
            limit: Some(0),
        },
        timeout_ms: Some(30_000),
    };
    agent_desktop_core::commands::find::execute(args, adapter, &CommandContext::default())
        .map(|_| ())
        .map_err(|error| format!("{error:?}"))
}

fn search_miss(entry: &RefEntry) -> RefEntry {
    let mut missing = search_entry(entry);
    missing.identity.name = Some("no such element anywhere".to_owned());
    missing
}

#[test]
fn a_search_miss_probe_drops_retained_object_identity() {
    use super::resolve::resolve;
    use super::resolve_fake::{FRAME_PATH, Fake, deadline, node};
    use super::role_numbers::PUSH_BUTTON;

    let mut fake = Fake::new();
    fake.add(FRAME_PATH, "/button", node(PUSH_BUTTON, "Save", 30, 20));
    let mut entry = fake.entry_of("/button", true);
    entry.identity.retained_object = Some("/button".to_owned());
    let mut pinned = entry.clone();
    pinned.scope.path.clear();
    pinned.identity.name = Some("no such element anywhere".to_owned());
    assert_eq!(
        resolve(&mut fake, &pinned, deadline())
            .expect("the exact object remains")
            .path(),
        "/button"
    );
    let error = resolve(&mut fake, &search_miss(&entry), deadline())
        .expect_err("a name-only search miss cannot retain the original object");
    assert_eq!(error.code, ErrorCode::StaleRef);
    assert_eq!(
        error.details.as_ref().expect("a classified miss")["kind"],
        "resolve_no_candidate"
    );
    fake.add(FRAME_PATH, "/twin", node(PUSH_BUTTON, "Save", 30, 20));
    let error = resolve(&mut fake, &search_entry(&entry), deadline())
        .expect_err("equal twins without an object anchor must be ambiguous");
    assert_eq!(error.code, ErrorCode::AmbiguousTarget);
}

fn perf_check(
    label: &str,
    adapter: &LinuxAdapter,
    bus: &SharedBus,
    frame: &AtspiElement,
) -> Result<(), String> {
    if !["files", "calculator", "vscode"].contains(&label) {
        return Ok(());
    }
    let rig = rig();
    let entries = rig.entries(frame, true);
    let window = window_id::encode(frame.bus_name(), frame.path()).ok_or("no window id")?;
    let unjudged = super::live_churn::unjudged(label, &entries);
    let (target, entry): (&String, &RefEntry) = entries
        .iter()
        .filter(|(id, _)| !unjudged.contains(id))
        .filter(|(_, entry)| entry.identity.role == "button" && entry.identity.name.is_some())
        .map(|(id, entry)| (id, entry))
        .next_back()
        .ok_or("no named button")?;
    println!("perf {label}: refs={} target={target}", entries.len());

    let hit = measure(|| {
        adapter
            .resolve_element_strict(entry, deadline(5_000))
            .expect("the path tier resolves");
    });
    report(label, "path-tier hit", &hit);

    let mut moved = entry.clone();
    moved.scope.path.clear();
    if label == "files" {
        println!(
            "perf files search resolving by identity and size: not measured, the search reads every button and Files mount rows churn"
        );
    } else {
        let mut outcomes = Vec::new();
        let sized =
            measure(|| outcomes.push(adapter.resolve_element_strict(&moved, deadline(5_000))));
        report(label, "search resolving by identity and size", &sized);
        if let Some(error) = outcomes
            .iter()
            .filter_map(|outcome| outcome.as_ref().err())
            .find(|error| error.code != ErrorCode::AmbiguousTarget)
        {
            return Err(format!("the search did not resolve: {error:?}"));
        }
    }

    let missing = search_miss(entry);
    let mut misses = Vec::new();
    let miss = measure(|| misses.push(adapter.resolve_element_strict(&missing, deadline(5_000))));
    report(label, "search miss", &miss);
    if misses.iter().any(Result::is_ok) {
        return Err("a name nothing has resolved to an element".to_owned());
    }

    let handle = adapter
        .resolve_element_strict(entry, deadline(5_000))
        .map_err(|error| error.message)?;
    let read = measure(|| {
        adapter
            .get_live_element(&handle, deadline(5_000))
            .expect("the element reads");
    });
    report(label, "get_live_element", &read);

    let connection = bus.get(deadline(5_000)).map_err(|error| error.message)?;
    let _ = connection;
    let walk = measure(|| {
        walk_tree(bus, frame, &WalkLimits::default(), deadline(10_000)).expect("the walk");
    });
    report(label, "snapshot walk (upper bound of a search)", &walk);

    let gate = measure(|| {
        let mut source = BusSource::open(bus, deadline(5_000)).expect("the bus opens");
        source.open_window(entry).expect("the gate admits");
    });
    report(label, "by-id window gate", &gate);

    let listing = adapter
        .list_windows(&WindowFilter::default(), deadline(10_000))
        .map_err(|error| error.message)?;
    let stored = listing
        .iter()
        .find(|candidate| candidate.id == window)
        .ok_or("the window is listed")?
        .clone();
    let strict = measure(|| {
        resolve_window_strict(bus, &stored, deadline(10_000)).expect("the window verifies");
    });
    report(label, "resolve_window_strict (listing)", &strict);

    if label == "files" {
        println!(
            "perf files find --role button (all, hydrated): not measured, Files rows churn and core's find stops on churn"
        );
        return Ok(());
    }
    let mut refusals = Vec::new();
    let found = measure(|| {
        if let Err(error) = find_all(adapter, &window, "button") {
            refusals.push(error);
        }
    });
    report(label, "find --role button (all, hydrated)", &found);
    match refusals.first() {
        Some(error) => Err(format!("find --role button answered an error: {error}")),
        None => Ok(()),
    }
}

#[test]
#[ignore = "measures real desktop applications; run through scripts/private-session.sh"]
fn perf_live_costs_min_of_seven_with_the_warm_up_discarded() {
    let adapter = LinuxAdapter::new();
    let bus = SharedBus::default();
    let failures: Vec<String> = each_app(&adapter, &bus, perf_check)
        .into_iter()
        .filter_map(|(label, outcome)| outcome.err().map(|error| format!("[{label}] {error}")))
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
