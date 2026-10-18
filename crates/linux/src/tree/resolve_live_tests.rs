use agent_desktop_core::{
    AdapterError, Deadline, ObservationOps, RefEntry, TreeOptions, WindowFilter, snapshot,
};

use super::live_launch::{Launched, nonce, program_of, vscode_arguments};
use super::live_qt;
use super::live_tree::{frames_where, pid_frame};
use super::node_read::read_node;
use super::walker_live_tests::settle;
use super::{AtspiElement, LabelOutcome, atspi_element, resolve_vocabulary};
use crate::LinuxAdapter;
use crate::system::{SharedBus, window_id, within};

const TERMINAL_SERVER: &str = "gnome-terminal-server";

#[derive(Default)]
struct Tally {
    resolved: usize,
    unverifiable: usize,
    failures: Vec<String>,
}

fn kind(error: &AdapterError) -> String {
    error
        .details
        .as_ref()
        .and_then(|details| details.get("kind"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn entries(result: &snapshot::SnapshotResult) -> Vec<(String, RefEntry)> {
    let value = serde_json::to_value(&result.refmap).expect("a refmap serializes");
    let inner = value["inner"]
        .as_object()
        .expect("a refmap holds its entries");
    let mut found: Vec<(String, RefEntry)> = inner
        .iter()
        .map(|(id, entry)| {
            (
                id.clone(),
                serde_json::from_value(entry.clone()).expect("an entry deserializes"),
            )
        })
        .collect();
    found.sort_by_key(|(id, _)| id.trim_start_matches("@e").parse::<u32>().unwrap_or(0));
    found
}

fn independent_read_disagrees(
    bus: &SharedBus,
    handle: &AtspiElement,
    entry: &RefEntry,
) -> Option<String> {
    let deadline = Deadline::after(5_000).expect("the deadline builds");
    let connection = bus.get(deadline).expect("the bus is reachable");
    let fresh = AtspiElement::new(handle.bus_name(), handle.path(), handle.pid());
    let raw = within(deadline, read_node(connection.inner(), &fresh))
        .map_err(|error| format!("second read failed: {}", error.message))
        .ok()?;
    let vocabulary = resolve_vocabulary(&raw, &LabelOutcome::Unlabelled, false);
    if let Some(role) = vocabulary.role.known()
        && role != &entry.identity.role
    {
        return Some(format!(
            "role {role} against stored {}",
            entry.identity.role
        ));
    }
    if let (Some(name), Some(stored)) = (raw.properties.name.non_blank(), &entry.identity.name)
        && name != stored
    {
        return Some(format!("name {name:?} against stored {stored:?}"));
    }
    if let (Some(stored), Some(extents)) = (entry.geometry.bounds, raw.extents.present())
        && agent_desktop_core::Rect::from(*extents) != stored
    {
        return Some(format!("bounds {extents:?} against stored {stored:?}"));
    }
    None
}

pub(super) fn snapshot_entries(
    adapter: &LinuxAdapter,
    frame: &AtspiElement,
    include_bounds: bool,
) -> Result<Vec<(String, RefEntry)>, String> {
    let deadline = Deadline::after(15_000).map_err(|error| error.message)?;
    let id = window_id::encode(frame.bus_name(), frame.path()).ok_or("the frame has no id")?;
    let windows = adapter
        .list_windows(&WindowFilter::default(), deadline)
        .map_err(|error| error.message)?;
    let window = windows
        .into_iter()
        .find(|window| window.id == id)
        .ok_or_else(|| format!("window {id} is not listed"))?;
    let options = TreeOptions {
        include_bounds,
        ..TreeOptions::default()
    };
    let result = snapshot::build(adapter, &options, None, Some(&window.id), deadline)
        .map_err(|error| error.to_string())?;
    Ok(entries(&result))
}

fn resolve_all(
    adapter: &LinuxAdapter,
    bus: &SharedBus,
    frame: &AtspiElement,
    include_bounds: bool,
) -> Result<Tally, String> {
    let mut tally = Tally::default();
    for (ref_id, entry) in snapshot_entries(adapter, frame, include_bounds)? {
        let attempt = Deadline::after(5_000).map_err(|error| error.message)?;
        match adapter.resolve_element_strict(&entry, attempt) {
            Ok(handle) => {
                let element = atspi_element(&handle).map_err(|error| error.message)?;
                match independent_read_disagrees(bus, element, &entry) {
                    None => tally.resolved += 1,
                    Some(difference) => tally.failures.push(format!(
                        "{ref_id} {} {:?} resolved to a node that differs: {difference}",
                        entry.identity.role, entry.identity.name
                    )),
                }
            }
            Err(error) if kind(&error) == "resolve_unverifiable_entry" => {
                println!(
                    "resolve_live unverifiable: {ref_id} role={} path={:?} bounds={:?}",
                    entry.identity.role, entry.scope.path, entry.geometry.bounds
                );
                tally.unverifiable += 1;
            }
            Err(error) => tally.failures.push(format!(
                "{ref_id} {} {:?} path {:?}: {:?} {} {}",
                entry.identity.role,
                entry.identity.name,
                entry.scope.path,
                error.code,
                kind(&error),
                error.message
            )),
        }
    }
    Ok(tally)
}

fn check(
    label: &str,
    adapter: &LinuxAdapter,
    bus: &SharedBus,
    frame: &AtspiElement,
) -> Result<(), String> {
    let mut failures = Vec::new();
    for include_bounds in [false, true] {
        let tally = resolve_all(adapter, bus, frame, include_bounds)?;
        println!(
            "resolve_live {label} bounds={include_bounds}: resolved={} unverifiable={} failed={}",
            tally.resolved,
            tally.unverifiable,
            tally.failures.len()
        );
        if tally.resolved == 0 {
            failures.push(format!("bounds={include_bounds}: no ref resolved"));
        }
        failures.extend(
            tally
                .failures
                .into_iter()
                .map(|line| format!("bounds={include_bounds}: {line}")),
        );
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(format!("[{label}] {}", failures.join("\n")))
    }
}

fn app(
    label: &str,
    program: &str,
    arguments: &[&str],
    min_nodes: usize,
    adapter: &LinuxAdapter,
    bus: &SharedBus,
) -> Result<(), String> {
    let connection = bus
        .get(Deadline::standard().map_err(|error| error.message)?)
        .map_err(|error| error.message)?;
    let mut launched = Launched::new();
    let pid = launched.spawn(program, arguments);
    let settled = settle(bus, min_nodes, || pid_frame(&connection, pid))
        .map_err(|error| format!("[{label}] {error}"))?;
    check(label, adapter, bus, &settled.frame)
}

fn terminal(adapter: &LinuxAdapter, bus: &SharedBus) -> Result<(), String> {
    let connection = bus
        .get(Deadline::standard().map_err(|error| error.message)?)
        .map_err(|error| error.message)?;
    let is_server = |pid: u32| program_of(pid).as_deref() == Some(TERMINAL_SERVER);
    let known = frames_where(&connection, is_server);
    let mut launched = Launched::new();
    let guard = std::env::temp_dir().join(nonce("resolve-terminal"));
    std::fs::create_dir_all(&guard).map_err(|error| error.to_string())?;
    launched.scratch.push(guard.clone());
    let script = format!(
        "while [ -d '{}' ] && kill -0 {} 2>/dev/null; do sleep 0.5; done",
        guard.display(),
        std::process::id()
    );
    launched.spawn("gnome-terminal", &["--wait", "--", "bash", "-c", &script]);
    let settled = settle(bus, 20, || {
        frames_where(&connection, is_server)
            .into_iter()
            .find(|frame| {
                !known
                    .iter()
                    .any(|old| old.path() == frame.path() && old.bus_name() == frame.bus_name())
            })
    })
    .map_err(|error| format!("[terminal] {error}"))?;
    check("terminal", adapter, bus, &settled.frame)
}

fn qt(adapter: &LinuxAdapter, bus: &SharedBus) -> Result<(), String> {
    let connection = bus
        .get(Deadline::standard().map_err(|error| error.message)?)
        .map_err(|error| error.message)?;
    let mut launched = Launched::new();
    let pid = live_qt::launch(&mut launched)?;
    let settled =
        settle(bus, 15, || pid_frame(&connection, pid)).map_err(|error| format!("[qt] {error}"))?;
    check("qt", adapter, bus, &settled.frame)
}

fn vscode(adapter: &LinuxAdapter, bus: &SharedBus) -> Result<(), String> {
    let home = std::env::var("HOME").map_err(|error| error.to_string())?;
    let program = format!("{home}/.local/ad-tools/vscode/app/code");
    if !std::path::Path::new(&program).exists() {
        return Err(format!("vscode is required and was not found at {program}"));
    }
    let data = std::env::temp_dir().join(nonce("resolve-vscode"));
    let data_arg = data.to_string_lossy().into_owned();
    let result = app(
        "vscode",
        &program,
        &vscode_arguments(&data_arg),
        200,
        adapter,
        bus,
    );
    let _ = std::fs::remove_dir_all(data);
    result
}

#[test]
#[ignore = "drives real desktop applications; run through scripts/private-session.sh"]
fn resolve_live_every_snapshot_ref_resolves_to_the_node_it_was_taken_from() {
    let adapter = LinuxAdapter::new();
    let bus = SharedBus::default();
    let outcomes = [
        (
            "files",
            app("files", "nautilus", &["--new-window"], 30, &adapter, &bus),
        ),
        (
            "calculator",
            app("calculator", "gnome-calculator", &[], 30, &adapter, &bus),
        ),
        (
            "text-editor",
            app(
                "text-editor",
                "gnome-text-editor",
                &["--new-window"],
                15,
                &adapter,
                &bus,
            ),
        ),
        ("terminal", terminal(&adapter, &bus)),
        ("qt", qt(&adapter, &bus)),
        ("vscode", vscode(&adapter, &bus)),
    ];
    let failures: Vec<String> = outcomes
        .into_iter()
        .filter_map(|(label, outcome)| outcome.err().map(|error| format!("[{label}] {error}")))
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
