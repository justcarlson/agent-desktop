use std::fmt::Write as _;
use std::time::{Duration, Instant};

use agent_desktop_core::{Deadline, ErrorCode, NativeHandle};
use zbus::blocking::Connection;

use super::live_launch::{Launched, nonce, processes_named, program_of};
use super::live_qt;
use super::walker::walk_tree;
use super::{AtspiElement, Field, RawNode, Walk, WalkLimits, WalkedNode, atspi_element};
use crate::system::SharedBus;

const REGISTRY_SERVICE: &str = "org.a11y.atspi.Registry";
const REGISTRY_ROOT: &str = "/org/a11y/atspi/accessible/root";
const ACCESSIBLE: &str = "org.a11y.atspi.Accessible";
const BUS_DAEMON: &str = "org.freedesktop.DBus";
const SETTLE_LIMIT: Duration = Duration::from_secs(40);
const POLL: Duration = Duration::from_millis(100);
const STABLE_POLLS: usize = 3;
const TIMED_RUNS: usize = 7;
const PUSH_BUTTON_ROLE: u32 = 43;
const TERMINAL_SERVER: &str = "gnome-terminal-server";

fn children_of(connection: &Connection, bus: &str, path: &str) -> Vec<(String, String)> {
    let Ok(reply) = connection.call_method(Some(bus), path, Some(ACCESSIBLE), "GetChildren", &())
    else {
        return Vec::new();
    };
    reply
        .body()
        .deserialize::<Vec<(String, zbus::zvariant::OwnedObjectPath)>>()
        .map(|entries| {
            entries
                .into_iter()
                .map(|(bus, path)| (bus, path.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

fn pid_of(connection: &Connection, bus: &str) -> Option<u32> {
    connection
        .call_method(
            Some(BUS_DAEMON),
            "/org/freedesktop/DBus",
            Some(BUS_DAEMON),
            "GetConnectionUnixProcessID",
            &(bus,),
        )
        .ok()?
        .body()
        .deserialize::<u32>()
        .ok()
}

fn frames_where(connection: &Connection, wanted: impl Fn(u32) -> bool) -> Vec<AtspiElement> {
    let mut frames = Vec::new();
    for (app_bus, _) in children_of(connection, REGISTRY_SERVICE, REGISTRY_ROOT) {
        let Some(pid) = pid_of(connection, &app_bus) else {
            continue;
        };
        if !wanted(pid) {
            continue;
        }
        for (frame_bus, frame_path) in children_of(connection, &app_bus, REGISTRY_ROOT) {
            frames.push(AtspiElement::new(frame_bus, frame_path, pid));
        }
    }
    frames
}

fn count_failed(node: &WalkedNode<RawNode>) -> usize {
    node.payload.failed_reads() + node.children.iter().map(count_failed).sum::<usize>()
}

fn dump(node: &WalkedNode<RawNode>, depth: usize, out: &mut String) {
    let payload = &node.payload;
    let role = payload
        .traits
        .role_name
        .present()
        .map_or("?", String::as_str);
    let name = payload.properties.name.present().map_or("", String::as_str);
    let _ = writeln!(
        out,
        "{}{role} {name:?} failed={} truncated={}",
        "  ".repeat(depth),
        payload.failed_reads(),
        node.truncated
    );
    for child in &node.children {
        dump(child, depth + 1, out);
    }
}

fn find_node<'a>(
    node: &'a WalkedNode<RawNode>,
    wanted: &impl Fn(&RawNode) -> bool,
) -> Option<&'a RawNode> {
    if wanted(&node.payload) {
        return Some(&node.payload);
    }
    node.children
        .iter()
        .find_map(|child| find_node(child, wanted))
}

fn action_summary(node: &WalkedNode<RawNode>) -> String {
    let mut found = Vec::new();
    collect_actions(node, &mut found);
    format!("{} nodes with an action field: {found:?}", found.len())
}

fn collect_actions(node: &WalkedNode<RawNode>, found: &mut Vec<String>) {
    if !matches!(node.payload.actions, Field::Absent) {
        found.push(format!("{:?}", node.payload.actions));
    }
    for child in &node.children {
        collect_actions(child, found);
    }
}

struct Settled {
    frame: AtspiElement,
    walk: Walk<RawNode>,
}

fn settle(
    bus: &SharedBus,
    min_nodes: usize,
    mut find_frame: impl FnMut() -> Option<AtspiElement>,
) -> Result<Settled, String> {
    let started = Instant::now();
    let mut last_count = 0;
    let mut stable = 0;
    let mut last_state = String::from("no frame appeared on the registry");
    while started.elapsed() < SETTLE_LIMIT {
        std::thread::sleep(POLL);
        let Some(frame) = find_frame() else { continue };
        let deadline = Deadline::after(5_000).map_err(|error| error.message)?;
        match walk_tree(bus, &frame, &WalkLimits::default(), deadline) {
            Ok(walk) => {
                last_state = format!("{} nodes, complete={}", walk.node_count, walk.complete);
                if walk.complete && walk.node_count >= min_nodes && walk.node_count == last_count {
                    stable += 1;
                } else {
                    stable = 0;
                }
                last_count = walk.node_count;
                if stable >= STABLE_POLLS {
                    return Ok(Settled { frame, walk });
                }
            }
            Err(error) => last_state = format!("walk failed: {}", error.message),
        }
    }
    Err(format!(
        "tree did not settle within the limit: {last_state}"
    ))
}

struct Timing {
    min: u128,
    median: u128,
    max: u128,
}

fn timed_walks(bus: &SharedBus, frame: &AtspiElement) -> Result<Timing, String> {
    let mut millis = Vec::new();
    for run in 0..=TIMED_RUNS {
        let deadline = Deadline::standard().map_err(|error| error.message)?;
        let started = Instant::now();
        walk_tree(bus, frame, &WalkLimits::default(), deadline).map_err(|error| error.message)?;
        if run > 0 {
            millis.push(started.elapsed().as_millis());
        }
    }
    millis.sort_unstable();
    Ok(Timing {
        min: millis[0],
        median: millis[millis.len() / 2],
        max: millis[millis.len() - 1],
    })
}

fn report(label: &str, settled: &Settled, timing: &Timing) -> usize {
    let failed = count_failed(&settled.walk.root);
    println!(
        "walker_live {label}: nodes={} depth={} complete={} read_errors={failed} ms min/median/max={}/{}/{}",
        settled.walk.node_count,
        settled.walk.max_depth,
        settled.walk.complete,
        timing.min,
        timing.median,
        timing.max
    );
    failed
}

fn walk_app(
    bus: &SharedBus,
    label: &str,
    min_nodes: usize,
    find_frame: impl FnMut() -> Option<AtspiElement>,
) -> Result<Settled, String> {
    let settled =
        settle(bus, min_nodes, find_frame).map_err(|error| format!("{label}: {error}"))?;
    let timing = timed_walks(bus, &settled.frame).map_err(|error| format!("{label}: {error}"))?;
    let failed = report(label, &settled, &timing);
    let mut problems = Vec::new();
    if !settled.walk.complete {
        problems.push("walk incomplete".to_owned());
    }
    if failed > 0 {
        problems.push(format!("{failed} read errors"));
    }
    if problems.is_empty() || label == "vscode" && settled.walk.complete {
        return Ok(settled);
    }
    let mut tree = String::new();
    dump(&settled.walk.root, 0, &mut tree);
    Err(format!("{label}: {}\n{tree}", problems.join(", ")))
}

fn pid_frame(connection: &Connection, pid: u32) -> Option<AtspiElement> {
    frames_where(connection, |candidate| candidate == pid)
        .into_iter()
        .next()
}

fn check_handles(frame: &AtspiElement) -> Result<(), String> {
    let error = atspi_element(&NativeHandle::null()).expect_err("a null handle must be rejected");
    let details = error.details.as_ref();
    let kind = details.and_then(|details| details.get("kind"));
    if error.code != ErrorCode::InvalidArgs || kind != Some(&"invalid_native_handle".into()) {
        return Err(format!("null handle gave {:?} {details:?}", error.code));
    }
    let handle = frame.clone().into_native_handle();
    match atspi_element(&handle) {
        Ok(element) if element == frame => Ok(()),
        other => Err(format!("handle did not round-trip: {other:?}")),
    }
}

fn check_gone(bus: &SharedBus, frame: &AtspiElement) -> Result<(), String> {
    let started = Instant::now();
    while started.elapsed() < SETTLE_LIMIT {
        let deadline = Deadline::standard().map_err(|error| error.message)?;
        if let Err(error) = walk_tree(bus, frame, &WalkLimits::default(), deadline) {
            return if matches!(
                error.code,
                ErrorCode::AppNotFound | ErrorCode::ElementNotFound
            ) {
                println!("walker_live exited app classified as {:?}", error.code);
                Ok(())
            } else {
                Err(format!(
                    "exited app classified as {:?}: {}",
                    error.code, error.message
                ))
            };
        }
        std::thread::sleep(POLL);
    }
    Err("an exited app still answered".to_owned())
}

fn files(bus: &SharedBus, connection: &Connection) -> Result<(), String> {
    let mut launched = Launched::new();
    let pid = launched.spawn("nautilus", &["--new-window"]);
    walk_app(bus, "files", 30, || pid_frame(connection, pid)).map(|_| ())
}

fn terminal(bus: &SharedBus, connection: &Connection) -> Result<(), String> {
    let is_server = |pid: u32| program_of(pid).as_deref() == Some(TERMINAL_SERVER);
    let known: Vec<AtspiElement> = frames_where(connection, is_server);
    let servers_before = processes_named(TERMINAL_SERVER);
    let marker = nonce("terminal");
    let mut launched = Launched::new();
    let script = format!("exec -a {marker} sleep 3600");
    launched.spawn("gnome-terminal", &["--wait", "--", "bash", "-c", &script]);
    let result = walk_app(bus, "terminal", 20, || {
        frames_where(connection, is_server)
            .into_iter()
            .find(|frame| {
                !known
                    .iter()
                    .any(|old| old.path() == frame.path() && old.bus_name() == frame.bus_name())
            })
    });
    launched.extra_pids.extend(processes_named(&marker));
    if servers_before.is_empty() {
        launched.extra_pids.extend(processes_named(TERMINAL_SERVER));
    }
    result.map(|_| ())
}

fn calculator(bus: &SharedBus, connection: &Connection) -> Result<(), String> {
    let mut launched = Launched::new();
    let pid = launched.spawn("gnome-calculator", &[]);
    let settled = walk_app(bus, "calculator", 30, || pid_frame(connection, pid))?;
    let button = find_node(&settled.walk.root, &|node| {
        node.actions
            .present()
            .is_some_and(|actions| !actions.is_empty())
    })
    .ok_or_else(|| {
        format!(
            "no calculator node reported a non-empty action list: {}",
            action_summary(&settled.walk.root)
        )
    })?;
    println!(
        "walker_live calculator action sample: role={:?} name={:?} actions={:?}",
        button.traits.role_name.present(),
        button.properties.name.present(),
        button.actions.present()
    );
    check_handles(&settled.frame)?;
    launched.stop();
    check_gone(bus, &settled.frame)
}

fn qt(bus: &SharedBus, connection: &Connection) -> Result<(), String> {
    let mut launched = Launched::new();
    let pid = live_qt::launch(&mut launched)?;
    let settled = walk_app(bus, "qt", 15, || pid_frame(connection, pid))?;
    let button = find_node(&settled.walk.root, &|node| {
        node.traits.role.present() == Some(&PUSH_BUTTON_ROLE)
            && node
                .properties
                .name
                .present()
                .is_some_and(|name| name == live_qt::BUTTON_LABEL)
    });
    match button {
        Some(node) => {
            println!(
                "walker_live qt button name={:?}",
                node.properties.name.present()
            );
            Ok(())
        }
        None => Err("qt: no push button named with a non-empty name".to_owned()),
    }
}

fn vscode(bus: &SharedBus, connection: &Connection) -> Result<(), String> {
    let home = std::env::var("HOME").map_err(|error| error.to_string())?;
    let program = format!("{home}/.local/ad-tools/vscode/app/code");
    if !std::path::Path::new(&program).exists() {
        return Err(format!("vscode is required and was not found at {program}"));
    }
    let mut launched = Launched::new();
    let data = std::env::temp_dir().join(nonce("vscode"));
    launched.scratch.push(data.clone());
    let data_arg = data.to_string_lossy().into_owned();
    let pid = launched.spawn(
        &program,
        &[
            "--no-sandbox",
            "--user-data-dir",
            &data_arg,
            "--disable-extensions",
            "--new-window",
        ],
    );
    walk_app(bus, "vscode", 200, || pid_frame(connection, pid)).map(|_| ())
}

#[test]
#[ignore = "drives real desktop applications; run through scripts/remote-linux.sh"]
fn walker_live_real_apps() {
    let bus = SharedBus::default();
    let connection = bus
        .get(Deadline::standard().expect("a standard deadline builds"))
        .expect("the accessibility bus must be reachable");
    let outcomes = [
        ("files", files(&bus, &connection)),
        ("terminal", terminal(&bus, &connection)),
        ("calculator", calculator(&bus, &connection)),
        ("qt", qt(&bus, &connection)),
        ("vscode", vscode(&bus, &connection)),
    ];
    let failures: Vec<String> = outcomes
        .into_iter()
        .filter_map(|(label, outcome)| outcome.err().map(|error| format!("[{label}] {error}")))
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
