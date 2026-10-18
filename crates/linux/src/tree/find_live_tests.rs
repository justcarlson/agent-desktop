use agent_desktop_core::commands::find::{FindArgs, FindFilterArgs, FindSelectionArgs, execute};
use agent_desktop_core::{AppError, CommandContext, ObservationOps, RefStore, SnapshotSurface};
use serde_json::Value;

use super::live_apps::each_app;
use super::resolve_live_tests::independent_read_disagrees;
use super::{AtspiElement, atspi_element};
use crate::LinuxAdapter;
use crate::system::{SharedBus, window_id};

/// The refusal core gives a selected match that has neither identity nor a
/// rectangle to anchor on, which stops the whole `find`.
const CORE_ANCHOR_REFUSAL: &str = "lacks a verifiable identity or geometry anchor";

fn args(window: &str, role: Option<&str>, name: Option<&str>, selection: Selection) -> FindArgs {
    FindArgs {
        app: None,
        window_id: Some(window.to_owned()),
        root: None,
        surface: SnapshotSurface::Window,
        filter: FindFilterArgs {
            role: role.map(str::to_owned),
            name: name.map(str::to_owned),
            description: None,
            native_id: None,
            value: None,
            text: None,
            exact: name.is_some(),
        },
        states: Vec::new(),
        selection: FindSelectionArgs {
            count: selection == Selection::Count,
            first: selection == Selection::First,
            last: false,
            nth: None,
            limit: (selection == Selection::All).then_some(0),
        },
        timeout_ms: Some(20_000),
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Selection {
    All,
    Count,
    First,
}

fn run(adapter: &LinuxAdapter, arguments: FindArgs) -> Result<Value, String> {
    let started = std::time::Instant::now();
    let outcome =
        execute(arguments, adapter, &CommandContext::default()).map_err(|error| match error {
            AppError::Adapter(error) => format!("{:?}: {}", error.code, error.message),
            other => other.to_string(),
        });
    println!(
        "find_live find took {} ms ok={}",
        started.elapsed().as_millis(),
        outcome.is_ok()
    );
    outcome
}

fn matches_of(data: &Value) -> Vec<&Value> {
    data["matches"]
        .as_array()
        .map(|found| found.iter().collect())
        .unwrap_or_default()
}

fn ref_back_to_its_node(
    adapter: &LinuxAdapter,
    bus: &SharedBus,
    found: &Value,
) -> Result<(), String> {
    let Some(ref_id) = found["ref_id"].as_str() else {
        return Ok(());
    };
    let entry = RefStore::new()
        .and_then(|store| store.load_ref(ref_id))
        .map_err(|error| format!("{ref_id} could not be loaded: {error}"))?;
    let deadline = agent_desktop_core::Deadline::after(10_000).map_err(|error| error.message)?;
    let handle = adapter
        .resolve_element_strict(&entry, deadline)
        .map_err(|error| {
            format!(
                "{ref_id} did not resolve: {:?} {}",
                error.code, error.message
            )
        })?;
    let element = atspi_element(&handle).map_err(|error| error.message)?;
    if found["role"].as_str() != Some(entry.identity.role.as_str()) {
        return Err(format!("{ref_id} role differs from its stored entry"));
    }
    match independent_read_disagrees(bus, element, &entry) {
        None => Ok(()),
        Some(difference) => Err(format!(
            "{ref_id} resolved to a node that differs: {difference}"
        )),
    }
}

fn find_check(
    label: &str,
    adapter: &LinuxAdapter,
    bus: &SharedBus,
    frame: &AtspiElement,
) -> Result<(), String> {
    let window = window_id::encode(frame.bus_name(), frame.path()).ok_or("the frame has no id")?;
    let counted = run(
        adapter,
        args(&window, Some("button"), None, Selection::Count),
    )?;
    let count = counted["count"].as_u64().ok_or("count is missing")? as usize;
    let listed = run(adapter, args(&window, Some("button"), None, Selection::All))?;
    let found = matches_of(&listed);
    println!(
        "find_live {label}: count={count} listed={} with_ref={}",
        found.len(),
        found.iter().filter(|m| m["ref_id"].is_string()).count()
    );
    let mut problems = Vec::new();
    if count == 0 {
        problems.push("find --count reported no button".to_owned());
    }
    if found.len() != count {
        problems.push(format!(
            "listed {} matches against count {count}",
            found.len()
        ));
    }
    for each in &found {
        if let Err(problem) = ref_back_to_its_node(adapter, bus, each) {
            problems.push(problem);
        }
    }
    match run(
        adapter,
        args(&window, Some("button"), None, Selection::First),
    ) {
        Ok(first) if first["match"].is_object() || first["ref_id"].is_string() => {}
        Ok(first) => problems.push(format!("--first returned no match: {first}")),
        Err(error) => problems.push(format!("--first failed: {error}")),
    }
    for role in ["listbox", "group", "tab", "list"] {
        match run(adapter, args(&window, Some(role), None, Selection::All)) {
            Ok(roles) => println!(
                "find_live {label} role {role}: {}",
                matches_of(&roles).len()
            ),
            Err(error) if error.contains(CORE_ANCHOR_REFUSAL) => {
                println!("find_live {label} role {role}: refused by core: {error}");
            }
            Err(error) => problems.push(format!("find --role {role} failed: {error}")),
        }
    }
    let nothing = format!("no-such-element-{}", std::process::id());
    match run(adapter, args(&window, None, Some(&nothing), Selection::All)) {
        Ok(empty) if matches_of(&empty).is_empty() => {}
        Ok(other) => problems.push(format!("a name nothing has matched: {other}")),
        Err(error) => problems.push(format!("a find with no match failed: {error}")),
    }
    if label == "files" {
        if let Err(problem) = view_options(adapter, bus, &window) {
            problems.push(problem);
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("\n"))
    }
}

fn view_options(adapter: &LinuxAdapter, bus: &SharedBus, window: &str) -> Result<(), String> {
    let listed = run(
        adapter,
        args(window, None, Some("View Options"), Selection::All),
    )?;
    let found = matches_of(&listed);
    let mut roles: Vec<&str> = found.iter().filter_map(|m| m["role"].as_str()).collect();
    roles.sort_unstable();
    println!("find_live files View Options roles={roles:?}");
    let mut refs: Vec<&str> = found.iter().filter_map(|m| m["ref_id"].as_str()).collect();
    refs.sort_unstable();
    refs.dedup();
    if found.len() < 2 || refs.len() != found.len() {
        return Err(format!(
            "View Options gave {} matches with {} distinct refs",
            found.len(),
            refs.len()
        ));
    }
    found
        .iter()
        .try_for_each(|each| ref_back_to_its_node(adapter, bus, each))
}

#[test]
#[ignore = "drives real desktop applications; run through scripts/private-session.sh"]
fn find_live_returns_matches_and_refs_that_resolve_back_to_their_nodes() {
    let adapter = LinuxAdapter::new();
    let bus = SharedBus::default();
    let failures: Vec<String> = each_app(&adapter, &bus, find_check)
        .into_iter()
        .filter_map(|(label, outcome)| outcome.err().map(|error| format!("[{label}] {error}")))
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
