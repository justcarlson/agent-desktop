use agent_desktop_core::{Deadline, ErrorCode, NativeHandle, ObservationOps, SystemOps};

use super::live_launch::{Launched, nonce};
use super::resolve_live_tests::snapshot_entries;
use super::{AtspiElement, atspi_element};
use crate::LinuxAdapter;
use crate::actions::scroll_into_view;
use crate::system::SharedBus;

#[path = "live_scroll_scripts.rs"]
pub(in crate::tree) mod scripts;

use scripts::{GTK3_LIST, QT_LIST, intersects, pointer, window_rect};

pub(in crate::tree) struct Target {
    pub(in crate::tree) name: String,
    pub(in crate::tree) handle: NativeHandle,
    pub(in crate::tree) element: AtspiElement,
}

pub(in crate::tree) fn nodes_of(
    adapter: &LinuxAdapter,
    _bus: &SharedBus,
    frame: &AtspiElement,
) -> Result<Vec<Target>, String> {
    let entries = snapshot_entries(adapter, frame, true)?;
    let mut found = Vec::new();
    for (_, entry) in entries {
        let Ok(handle) =
            adapter.resolve_element_strict(&entry, Deadline::after(5_000).expect("deadline"))
        else {
            continue;
        };
        let Ok(element) = atspi_element(&handle).cloned() else {
            continue;
        };
        found.push(Target {
            name: entry.identity.name.clone().unwrap_or_default(),
            handle,
            element,
        });
    }
    Ok(found)
}

fn scroll_case(
    label: &str,
    adapter: &LinuxAdapter,
    bus: &SharedBus,
    frame: &AtspiElement,
    wanted: &str,
    expect_realised: bool,
) -> Result<(), String> {
    let (_, entry) = snapshot_entries(adapter, frame, true)?
        .into_iter()
        .find(|(_, entry)| entry.identity.name.as_deref() == Some(wanted))
        .ok_or_else(|| format!("[{label}] no target {wanted:?}"))?;
    let handle = adapter
        .resolve_element_strict(&entry, Deadline::after(5_000).expect("deadline"))
        .map_err(|error| error.message)?;
    let element = atspi_element(&handle).map_err(|error| error.message)?;
    let deadline = Deadline::after(10_000).map_err(|error| error.message)?;
    adapter
        .get_live_state(&handle, deadline)
        .map_err(|error| error.message)?
        .ok_or("no state")?;
    let lease = SystemOps::acquire_interaction_lease(adapter, deadline).map_err(|e| e.message)?;
    let pointer_before = pointer().ok_or("the private X11 pointer is unreadable")?;
    let outcome = scroll_into_view(bus, &handle, &lease);
    let pointer_after = pointer().ok_or("the private X11 pointer is unreadable after scrolling")?;
    let frame_rect = window_rect(bus, frame);
    let own = window_rect(bus, element);
    assert_eq!(pointer_before, pointer_after, "[{label}] the pointer moved");
    if expect_realised {
        outcome.map_err(|error| format!("[{label}] {:?} {}", error.code, error.message))?;
        if intersects(own, frame_rect) != Some(true) {
            return Err(format!(
                "[{label}] the row's own extents {own:?} are outside {frame_rect:?}"
            ));
        }
    } else {
        let error = outcome
            .err()
            .ok_or_else(|| format!("[{label}] a refusal was expected"))?;
        assert!(
            matches!(
                error.code,
                ErrorCode::ActionFailed | ErrorCode::ActionNotSupported
            ),
            "[{label}] {error:?}"
        );
        let details = error.details.as_ref().expect("a refusal carries details");
        assert_eq!(details["physical_delivery_started"], false);
        assert_eq!(details["kind"], "scroll_into_view");
    }
    Ok(())
}

fn script_app(
    label: &str,
    program: &str,
    source: &str,
    env: &[(&str, String)],
    wanted: &'static str,
    expect_realised: bool,
) {
    let adapter = LinuxAdapter::new();
    let bus = SharedBus::default();
    let dir = std::env::temp_dir().join(nonce(label));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let script = dir.join("app.py");
    std::fs::write(&script, source).expect("script");
    let connection = bus
        .get(Deadline::standard().expect("deadline"))
        .expect("bus");
    let mut launched = Launched::new();
    launched.scratch.push(dir);
    let script_arg = script.to_string_lossy().into_owned();
    let pid = launched.spawn_with_env(program, &[&script_arg], env);
    let settled = super::walker_live_tests::settle(&bus, 40, || {
        super::live_tree::pid_frame(&connection, pid)
    })
    .expect("the app settles");
    scroll_case(
        label,
        &adapter,
        &bus,
        &settled.frame,
        wanted,
        expect_realised,
    )
    .expect("the case holds");
}

#[test]
#[ignore = "drives a real desktop application; run through scripts/private-session.sh"]
fn offscreen_live_gtk3_scrolled_list_realises_an_offscreen_row() {
    script_app("gtk3", "python3", GTK3_LIST, &[], "Row 250", true);
}

#[test]
#[ignore = "drives a real desktop application; run through scripts/private-session.sh"]
fn offscreen_live_qt_list_item_cannot_be_scrolled_to() {
    let home = std::env::var("HOME").expect("HOME");
    let python = format!("{home}/.local/ad-tools/qtvenv/bin/python");
    let libraries = format!("{home}/.local/ad-tools/root/usr/lib/x86_64-linux-gnu");
    let env = [
        ("QT_ACCESSIBILITY", "1".to_owned()),
        ("QT_LINUX_ACCESSIBILITY_ALWAYS_ON", "1".to_owned()),
        ("QT_QPA_PLATFORM", "xcb".to_owned()),
        ("LD_LIBRARY_PATH", libraries),
    ];
    script_app("qt", &python, QT_LIST, &env, "Item 250", false);
}
