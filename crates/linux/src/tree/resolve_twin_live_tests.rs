use agent_desktop_core::{AdapterError, Deadline, ObservationOps, RefEntry};

use super::live_launch::Launched;
use super::live_qt;
use super::live_tree::pid_frame;
use super::node_read::read_node;
use super::resolve_live_tests::snapshot_entries;
use super::walker_live_tests::settle;
use super::{AtspiElement, atspi_element};
use crate::LinuxAdapter;
use crate::system::{SharedBus, within};

fn width_of(bus: &SharedBus, handle: &agent_desktop_core::NativeHandle) -> f64 {
    let element = atspi_element(handle).expect("a resolved handle is ours");
    let deadline = Deadline::after(5_000).expect("the deadline builds");
    let connection = bus.get(deadline).expect("the bus is reachable");
    let fresh = AtspiElement::new(element.bus_name(), element.path(), element.pid());
    let raw = within(deadline, read_node(connection.inner(), &fresh)).expect("a second read");
    f64::from(raw.extents.present().expect("the node has extents").width)
}

fn resolve_widths(
    adapter: &LinuxAdapter,
    bus: &SharedBus,
    entries: &[RefEntry],
) -> Result<Vec<f64>, AdapterError> {
    entries
        .iter()
        .map(|entry| {
            let deadline = Deadline::after(5_000).expect("the deadline builds");
            adapter
                .resolve_element_strict(entry, deadline)
                .map(|handle| width_of(bus, &handle))
        })
        .collect()
}

fn without_path(entries: &[RefEntry]) -> Vec<RefEntry> {
    entries
        .iter()
        .cloned()
        .map(|mut entry| {
            entry.scope.path.clear();
            entry
        })
        .collect()
}

fn without_size(entries: &[RefEntry]) -> Vec<RefEntry> {
    entries
        .iter()
        .cloned()
        .map(|mut entry| {
            entry.geometry.bounds = None;
            entry.geometry.bounds_hash = None;
            entry
        })
        .collect()
}

fn named(entries: Vec<(String, RefEntry)>, name: &str) -> Vec<RefEntry> {
    entries
        .into_iter()
        .map(|(_, entry)| entry)
        .filter(|entry| entry.identity.name.as_deref() == Some(name))
        .collect()
}

fn stored_widths(entries: &[RefEntry]) -> Vec<f64> {
    entries
        .iter()
        .map(|entry| entry.geometry.bounds.expect("a rectangle was kept").width)
        .collect()
}

#[test]
#[ignore = "drives a real desktop application; run through scripts/private-session.sh"]
fn resolve_live_files_view_options_nodes_each_resolve_to_their_own_node() {
    let adapter = LinuxAdapter::new();
    let bus = SharedBus::default();
    let connection = bus
        .get(Deadline::standard().expect("a standard deadline builds"))
        .expect("the accessibility bus must be reachable");
    let mut launched = Launched::new();
    let pid = launched.spawn("nautilus", &["--new-window"]);
    let settled = settle(&bus, 30, || pid_frame(&connection, pid)).expect("files settles");
    let snapshot = |include_bounds| {
        named(
            snapshot_entries(&adapter, &settled.frame, include_bounds).expect("a snapshot"),
            "View Options",
        )
    };
    let sized = snapshot(true);
    let roles: Vec<&str> = sized
        .iter()
        .map(|entry| entry.identity.role.as_str())
        .collect();
    println!(
        "resolve_live files View Options roles={roles:?} widths={:?}",
        stored_widths(&sized)
    );
    assert!(
        sized.len() >= 2,
        "Files shows the View Options buttons: {roles:?}"
    );
    let stored = stored_widths(&sized);
    assert_eq!(
        resolve_widths(&adapter, &bus, &sized).expect("by path"),
        stored
    );
    assert_eq!(
        resolve_widths(&adapter, &bus, &without_path(&sized)).expect("by identity and size"),
        stored
    );
    let default_mode = snapshot(false);
    assert!(
        default_mode
            .iter()
            .all(|entry| entry.geometry.bounds.is_none())
    );
    assert_eq!(
        resolve_widths(&adapter, &bus, &default_mode).expect("by path"),
        stored
    );
}

#[test]
#[ignore = "drives a real desktop application; run through scripts/private-session.sh"]
fn resolve_live_twin_buttons_are_told_apart_by_path_and_by_stored_size_and_stay_ambiguous_without()
{
    let adapter = LinuxAdapter::new();
    let bus = SharedBus::default();
    let connection = bus
        .get(Deadline::standard().expect("a standard deadline builds"))
        .expect("the accessibility bus must be reachable");
    let mut launched = Launched::new();
    let pid = live_qt::launch_twins(&mut launched).expect("the twin window starts");
    let settled = settle(&bus, 4, || pid_frame(&connection, pid)).expect("the twins settle");
    let sized = named(
        snapshot_entries(&adapter, &settled.frame, true).expect("a snapshot"),
        "Twin",
    );
    assert_eq!(sized.len(), 2, "two twin buttons: {sized:?}");
    let stored = stored_widths(&sized);
    assert_ne!(stored[0], stored[1], "the twins differ in width");
    assert_eq!(
        resolve_widths(&adapter, &bus, &sized).expect("by path"),
        stored
    );
    assert_eq!(
        resolve_widths(&adapter, &bus, &without_path(&sized)).expect("by stored size"),
        stored
    );
    let deadline = Deadline::after(5_000).expect("the deadline builds");
    let nothing = without_size(&without_path(&sized));
    let Err(error) = adapter.resolve_element_strict(&nothing[0], deadline) else {
        panic!("nothing separates the twins, so the ref cannot resolve");
    };
    assert_eq!(error.code, agent_desktop_core::ErrorCode::AmbiguousTarget);
    assert_eq!(
        error.details.as_ref().expect("details")["candidate_count"],
        2
    );

    let default_mode = named(
        snapshot_entries(&adapter, &settled.frame, false).expect("a snapshot"),
        "Twin",
    );
    assert_eq!(default_mode.len(), 2);
    assert!(
        default_mode
            .iter()
            .all(|entry| entry.geometry.bounds.is_none())
    );
    assert_eq!(
        resolve_widths(&adapter, &bus, &default_mode).expect("by path alone"),
        stored
    );
}
