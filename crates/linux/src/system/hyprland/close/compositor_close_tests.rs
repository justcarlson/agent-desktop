use std::io::{Read, Write};
use std::os::unix::net::UnixListener;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use super::*;
use crate::system::hyprland::protocol::MAPPING_INTERFACE;
use crate::system::hyprland::toplevel::fake_binding::binding_for;
use crate::system::process::process_facts;
use crate::system::process::process_lookup::Lookup;
use crate::system::wayland::fake_compositor::{FakeCompositor, Script};
use crate::system::wayland::fake_toplevels::FakeToplevel;
use crate::system::wayland::wlr_toplevels::{SEAT_INTERFACE, WLR_MANAGER_INTERFACE};

const OTHER_PID: u32 = 1;
const STALL: Duration = Duration::from_millis(2_500);

fn serve_clients(path: &std::path::Path, third_owners: Vec<u32>, stall: Arc<AtomicBool>) {
    let own = std::process::id();
    let _ = std::fs::remove_file(path);
    let listener = UnixListener::bind(path).expect("bind");
    let client = |address: &str, pid: u32| {
        format!(
            r#"{{"address":"{address}","pid":{pid},"at":[0,0],"size":[1,1],"workspace":{{"name":"1"}},"mapped":true}}"#
        )
    };
    let bodies: Vec<String> = third_owners
        .into_iter()
        .map(|third| {
            format!(
                "[{},{},{}]",
                client("0x10", own),
                client("0x20", OTHER_PID),
                client("0x30", third)
            )
        })
        .collect();
    std::thread::spawn(move || {
        for (served, stream) in listener.incoming().enumerate() {
            let Ok(mut stream) = stream else { break };
            let mut command = [0_u8; 64];
            let _ = stream.read(&mut command);
            if stall.load(Ordering::SeqCst) {
                std::thread::sleep(STALL);
            }
            let body = &bodies[served.min(bodies.len() - 1)];
            let _ = stream.write_all(body.as_bytes());
        }
    });
}

fn own_instance() -> String {
    match process_facts::lookup(std::process::id()) {
        Lookup::Found(facts) => facts.instance,
        _ => panic!("own process is readable"),
    }
}

fn open() -> (FakeCompositor, CompositorClose) {
    open_with(vec![std::process::id()])
}

fn open_with(third_owners: Vec<u32>) -> (FakeCompositor, CompositorClose) {
    let (compositor, close, _) = open_until(third_owners, 2_000);
    (compositor, close)
}

fn open_until(
    third_owners: Vec<u32>,
    deadline_ms: u64,
) -> (FakeCompositor, CompositorClose, Arc<AtomicBool>) {
    let toplevel = |identifier, address, parent| FakeToplevel {
        identifier,
        address: Some(address),
        parent,
        ..FakeToplevel::default()
    };
    let compositor = FakeCompositor::start(Script::Toplevels {
        globals: vec![
            (SEAT_INTERFACE, 7),
            (WLR_MANAGER_INTERFACE, 3),
            (MAPPING_INTERFACE, 1),
        ],
        toplevels: vec![
            toplevel("one", 0x10, None),
            toplevel("two", 0x20, None),
            toplevel("three", 0x30, Some(0)),
        ],
    });
    let binding = binding_for(&compositor);
    let stall = Arc::new(AtomicBool::new(false));
    serve_clients(&binding.ipc_socket, third_owners, Arc::clone(&stall));
    let deadline = Deadline::after(deadline_ms).expect("deadline");
    let close = CompositorClose::open(binding, std::process::id(), deadline).expect("opens");
    (compositor, close, stall)
}

fn ids(read: ControlRead<Vec<Toplevel>>) -> Vec<(String, bool)> {
    read.ready()
        .expect("ready")
        .into_iter()
        .map(|toplevel| (toplevel.title, toplevel.transient))
        .collect()
}

#[test]
fn the_processs_toplevels_are_read_with_the_child_as_a_dialog() {
    let (_compositor, mut close) = open();

    let read = close.toplevels();

    let owned = read.ready().expect("ready");
    assert_eq!(owned.len(), 2);
    assert!(!owned[0].transient);
    assert!(owned[1].transient);
}

#[test]
fn an_address_that_changes_owner_across_the_toplevel_read_is_not_the_processs() {
    let (_compositor, mut close) = open_with(vec![std::process::id(), OTHER_PID]);

    let owned = ids(close.toplevels());

    assert_eq!(owned, vec![("one".to_owned(), false)]);
}

#[test]
fn a_close_reaches_only_a_toplevel_the_process_owns() {
    let (compositor, mut close) = open();
    let instance = own_instance();
    let owned: Vec<u32> = close
        .toplevels()
        .ready()
        .expect("ready")
        .iter()
        .map(|toplevel| toplevel.id)
        .collect();
    let foreign = owned[0] + 1;

    assert_eq!(close.request_close(owned[0], &instance), Ok(()));
    assert_eq!(
        close.request_close(foreign, &instance),
        Err(ControlError::StaleWindow)
    );
    assert_eq!(
        close.request_close(owned[0], &instance),
        Err(ControlError::BadWindow)
    );
    assert_eq!(
        close.request_close(owned[1], "boot:0"),
        Err(ControlError::StaleWindow)
    );
    assert_eq!(
        compositor.log.lock().expect("log").clone(),
        vec!["close one".to_owned()]
    );
    assert_eq!(ids(close.toplevels()).len(), 1);
}

#[test]
fn a_revalidation_read_that_runs_out_the_deadline_is_a_timeout_and_sends_nothing() {
    let (compositor, mut close, stall) = open_until(vec![std::process::id()], 1_000);
    let instance = own_instance();
    let first = close.toplevels().ready().expect("ready")[0].id;
    stall.store(true, Ordering::SeqCst);

    assert_eq!(
        close.request_close(first, &instance),
        Err(ControlError::TimedOut)
    );
    assert!(compositor.log.lock().expect("log").is_empty());
}
