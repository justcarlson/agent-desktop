use std::time::{Duration, Instant};

use agent_desktop_core::{ActionResult, AdapterError, Deadline, ErrorCode};
use zbus::blocking::Connection;

use super::click::{click, right_click};
use super::context::Target;
use crate::adapter::LinuxAdapter;
use crate::system::{ACCESSIBLE_INTERFACE, REGISTRY_ROOT_PATH, REGISTRY_SERVICE};
use crate::tree::action_read::{NodeView, read_node_view};
use crate::tree::live_launch::Launched;
use crate::tree::role_numbers::PUSH_BUTTON;
use crate::tree::{AtspiElement, atspi_element};

const FRAME_WAIT: Duration = Duration::from_secs(30);
const SEARCH_NODES: usize = 4000;

pub(super) struct Session {
    adapter: LinuxAdapter,
    connection: Connection,
    frame: AtspiElement,
    _launched: Launched,
}

fn raw_children(connection: &Connection, bus: &str, path: &str) -> Vec<(String, String)> {
    let Ok(reply) = connection.call_method(
        Some(bus),
        path,
        Some(ACCESSIBLE_INTERFACE),
        "GetChildren",
        &(),
    ) else {
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

fn owner_pid(connection: &Connection, bus: &str) -> Option<u32> {
    connection
        .call_method(
            Some("org.freedesktop.DBus"),
            "/org/freedesktop/DBus",
            Some("org.freedesktop.DBus"),
            "GetConnectionUnixProcessID",
            &(bus,),
        )
        .ok()?
        .body()
        .deserialize::<u32>()
        .ok()
}

fn frame_of(connection: &Connection, pid: u32) -> Option<AtspiElement> {
    raw_children(connection, REGISTRY_SERVICE, REGISTRY_ROOT_PATH)
        .into_iter()
        .filter(|(bus, _)| owner_pid(connection, bus) == Some(pid))
        .find_map(|(bus, _)| {
            raw_children(connection, &bus, REGISTRY_ROOT_PATH)
                .into_iter()
                .next()
                .map(|(frame_bus, frame_path)| AtspiElement::new(frame_bus, frame_path, pid))
        })
}

pub(super) fn open(program: &str, arguments: &[&str]) -> Result<Session, String> {
    let adapter = LinuxAdapter::new();
    let deadline = Deadline::standard().map_err(|error| error.message)?;
    let connection = adapter.bus().get(deadline).map_err(|error| error.message)?;
    let mut launched = Launched::new();
    let pid = launched.spawn(program, arguments);
    let started = Instant::now();
    loop {
        if let Some(frame) = frame_of(&connection, pid) {
            std::thread::sleep(Duration::from_secs(2));
            return Ok(Session {
                adapter,
                connection,
                frame,
                _launched: launched,
            });
        }
        if started.elapsed() > FRAME_WAIT {
            return Err(format!("{program} showed no accessible frame"));
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

pub(super) fn find(
    session: &Session,
    wanted: impl Fn(&NodeView) -> bool,
) -> Result<AtspiElement, String> {
    let deadline = Deadline::standard().map_err(|error| error.message)?;
    let mut stack = vec![session.frame.clone()];
    let mut seen = 0;
    while let Some(element) = stack.pop() {
        seen += 1;
        if seen > SEARCH_NODES {
            break;
        }
        let handle = element.clone().into_native_handle();
        if let Ok(view) = read_node_view(session.adapter.bus(), &handle, deadline)
            && wanted(&view)
        {
            return Ok(element);
        }
        let kids = raw_children(&session.connection, element.bus_name(), element.path());
        for (bus, path) in kids.into_iter().rev() {
            stack.push(AtspiElement::new(bus, path, element.pid()));
        }
    }
    Err("no node matched".to_owned())
}

pub(super) fn find_named(session: &Session, role: u32, name: &str) -> Result<AtspiElement, String> {
    find(session, |view| {
        view.role() == Some(role) && view.name() == Some(name)
    })
}

pub(super) fn act(
    session: &Session,
    element: &AtspiElement,
    run: impl FnOnce(&Target<'_>) -> Result<ActionResult, AdapterError>,
) -> Result<ActionResult, AdapterError> {
    let handle = element.clone().into_native_handle();
    let target = Target {
        bus: session.adapter.bus(),
        handle: &handle,
        element: atspi_element(&handle)?,
        deadline: Deadline::standard()?,
    };
    run(&target)
}

pub(super) fn state(session: &Session, element: &AtspiElement, bit: u32) -> Option<bool> {
    let handle = element.clone().into_native_handle();
    let deadline = Deadline::standard().ok()?;
    read_node_view(session.adapter.bus(), &handle, deadline)
        .ok()?
        .state(bit)
}

#[test]
#[ignore = "live: needs the AT-SPI session, gnome-calculator and the write engine"]
fn a_calculator_digit_click_is_delivered_and_unverified_on_its_own_node() {
    let session = open("gnome-calculator", &[]).expect("calculator opens");
    let digit = find_named(&session, PUSH_BUTTON, "7").expect("digit 7");
    let result = act(&session, &digit, click).expect("click delivers");
    let steps = result.steps;
    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0].verified(), Some(false));
}

#[test]
#[ignore = "live: needs the AT-SPI session, gnome-calculator and the write engine"]
fn a_click_on_an_insensitive_undo_is_not_delivered() {
    let session = open("gnome-calculator", &[]).expect("calculator opens");
    let undo = find(&session, |view| {
        view.role() == Some(PUSH_BUTTON) && view.name().is_some_and(|name| name.contains("Undo"))
    })
    .expect("undo button");
    let error = act(&session, &undo, click).expect_err("an insensitive button takes no click");
    assert_ne!(error.code, ErrorCode::Internal);
}

#[test]
#[ignore = "live: needs the AT-SPI session, gnome-calculator and the write engine"]
fn right_click_without_a_context_menu_verb_is_not_supported() {
    let session = open("gnome-calculator", &[]).expect("calculator opens");
    let digit = find_named(&session, PUSH_BUTTON, "7").expect("digit 7");
    let error = act(&session, &digit, right_click).expect_err("no context menu verb");
    assert_eq!(error.code, ErrorCode::ActionNotSupported);
}

#[test]
#[ignore = "live: needs the AT-SPI session and gnome-calculator"]
fn headless_focus_is_denied_before_any_write() {
    let session = open("gnome-calculator", &[]).expect("calculator opens");
    let digit = find_named(&session, PUSH_BUTTON, "7").expect("digit 7");
    let error = act(&session, &digit, |target| {
        super::focus::focus(target, agent_desktop_core::InteractionPolicy::headless())
    })
    .expect_err("focus steal is denied");
    assert_eq!(error.code, ErrorCode::PolicyDenied);
}

#[test]
#[ignore = "live: needs the AT-SPI session, gnome-calculator and the write engine"]
fn headed_focus_on_a_gtk_button_without_a_focus_verb_is_not_supported() {
    let session = open("gnome-calculator", &[]).expect("calculator opens");
    let digit = find_named(&session, PUSH_BUTTON, "7").expect("digit 7");
    let error = act(&session, &digit, |target| {
        super::focus::focus(target, agent_desktop_core::InteractionPolicy::headed())
    })
    .expect_err("no setfocus verb on GTK");
    assert_eq!(error.code, ErrorCode::ActionNotSupported);
}
