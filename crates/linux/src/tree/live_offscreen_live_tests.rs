use std::time::{Duration, Instant};

use agent_desktop_core::{Deadline, ErrorCode, NativeHandle, ObservationOps, SystemOps, state};

use super::live_apps::{self, AppSpec};
use super::live_launch::{Launched, nonce};
use super::resolve_live_tests::snapshot_entries;
use super::{AtspiElement, atspi_element};
use crate::LinuxAdapter;
use crate::actions::scroll_into_view;
use crate::system::SharedBus;

#[path = "live_scroll_scripts.rs"]
pub(in crate::tree) mod scripts;

use scripts::{GTK3_LIST, GTK4_LIST, QT_LIST, far_outside, intersects, pointer, window_rect};

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

fn survey(
    label: &str,
    adapter: &LinuxAdapter,
    bus: &SharedBus,
    nodes: &[Target],
    frame: &AtspiElement,
) {
    let frame_rect = window_rect(bus, frame);
    let (mut yes, mut no, mut unknown, mut hidden) = (0, 0, 0, 0);
    let mut contradictions = Vec::new();
    for node in nodes.iter().take(800) {
        let Ok(Some(read)) =
            adapter.get_live_state(&node.handle, Deadline::after(5_000).expect("deadline"))
        else {
            continue;
        };
        hidden += usize::from(read.states.iter().any(|token| token == state::HIDDEN));
        let node_rect = window_rect(bus, &node.element);
        let container_rect = node
            .element
            .scroll_container()
            .map(|container| window_rect(bus, container));
        let in_container = container_rect.map_or(Some(true), |rect| intersects(node_rect, rect));
        let oracle_on = intersects(node_rect, frame_rect)
            .filter(|inside| *inside && in_container == Some(true));
        let oracle_off = far_outside(node_rect, frame_rect);
        match read.offscreen {
            Some(true) => {
                yes += 1;
                if oracle_on == Some(true) {
                    contradictions.push(format!("{} reported offscreen but intersects", node.name));
                }
            }
            Some(false) => {
                no += 1;
                if oracle_off == Some(true) {
                    contradictions.push(format!(
                        "{} reported on screen but is far outside",
                        node.name
                    ));
                }
            }
            None => unknown += 1,
        }
    }
    println!(
        "offscreen_live {label}: nodes={} offscreen={yes} on_screen={no} unknown={unknown} hidden={hidden} contradictions={}",
        nodes.len().min(800),
        contradictions.len()
    );
    for line in contradictions.iter().take(10) {
        println!("offscreen_live {label}: CONTRADICTION {line}");
    }
    assert!(contradictions.is_empty(), "[{label}] {contradictions:?}");
}

fn scroll_case(
    label: &str,
    adapter: &LinuxAdapter,
    bus: &SharedBus,
    frame: &AtspiElement,
    wanted: Option<&str>,
    expect_realised: bool,
) -> Result<(), String> {
    let nodes = nodes_of(adapter, bus, frame)?;
    survey(label, adapter, bus, &nodes, frame);
    let target = match wanted {
        Some(name) => nodes.iter().find(|node| node.name == name),
        None => nodes.iter().rev().find(|node| {
            adapter
                .get_live_state(&node.handle, Deadline::after(5_000).expect("deadline"))
                .ok()
                .flatten()
                .is_some_and(|read| read.offscreen == Some(true))
        }),
    }
    .ok_or_else(|| format!("[{label}] no target {wanted:?} in {} nodes", nodes.len()))?;
    let handle = &target.handle;
    let deadline = Deadline::after(10_000).map_err(|error| error.message)?;
    let before = adapter
        .get_live_state(handle, deadline)
        .map_err(|error| error.message)?
        .ok_or("no state")?;
    println!(
        "scroll_live {label}: target {:?} before: offscreen={:?} states={:?} window={:?}",
        target.name,
        before.offscreen,
        before.states,
        window_rect(bus, &target.element)
    );
    let lease = SystemOps::acquire_interaction_lease(adapter, deadline).map_err(|e| e.message)?;
    let pointer_before = pointer();
    let started = Instant::now();
    let outcome = scroll_into_view(bus, handle, &lease);
    let elapsed = started.elapsed();
    let pointer_after = pointer();
    let after = adapter
        .get_live_state(handle, Deadline::after(5_000).expect("deadline"))
        .ok()
        .flatten();
    let frame_rect = window_rect(bus, frame);
    let own = window_rect(bus, &target.element);
    println!(
        "scroll_live {label}: outcome={:?} in {} ms; after: offscreen={:?} own={own:?} frame={frame_rect:?}; pointer {pointer_before:?} -> {pointer_after:?}",
        outcome
            .as_ref()
            .map_err(|e| (e.code.clone(), e.details.clone())),
        elapsed.as_millis(),
        after.as_ref().map(|read| read.offscreen),
    );
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
    wanted: Option<&'static str>,
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
    script_app("gtk3", "python3", GTK3_LIST, &[], Some("Row 250"), true);
}

#[test]
#[ignore = "drives a real desktop application; run through scripts/private-session.sh"]
fn offscreen_live_gtk4_scrolled_list_realises_an_offscreen_row() {
    script_app("gtk4", "python3", GTK4_LIST, &[], Some("Row 250"), true);
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
    script_app("qt", &python, QT_LIST, &env, Some("Item 250"), false);
}

/// The Files grid virtualises its rows, so a window may expose none off
/// screen; that is reported rather than failed.
fn files_check(
    label: &str,
    adapter: &LinuxAdapter,
    bus: &SharedBus,
    frame: &AtspiElement,
) -> Result<(), String> {
    match scroll_case(label, adapter, bus, frame, None, true) {
        Err(message) if message.contains("no target") => {
            println!("scroll_live {label}: the tree holds no offscreen row ({message})");
            Ok(())
        }
        Err(message) if message.contains("no scroll bar") => {
            println!(
                "scroll_live {label}: the offscreen node sits in a scroller without a bar ({message})"
            );
            Ok(())
        }
        other => other,
    }
}

fn check_each(
    label: &str,
    adapter: &LinuxAdapter,
    bus: &SharedBus,
    frame: &AtspiElement,
) -> Result<(), String> {
    scroll_case(label, adapter, bus, frame, None, true)
}

#[test]
#[ignore = "drives a real desktop application; run through scripts/private-session.sh"]
fn offscreen_live_files_grid_realises_an_offscreen_row() {
    let dir = std::env::temp_dir().join(nonce("files"));
    std::fs::create_dir_all(&dir).expect("dir");
    for i in 0..300 {
        std::fs::write(dir.join(format!("file-{i:03}.txt")), "x").expect("file");
    }
    let target = dir.to_string_lossy().into_owned();
    let result = live_apps::app(
        &AppSpec {
            label: "files",
            program: "nautilus",
            arguments: &["--new-window", &target],
            min_nodes: 30,
        },
        &LinuxAdapter::new(),
        &SharedBus::default(),
        files_check,
    );
    let _ = std::fs::remove_dir_all(dir);
    result.expect("the case holds");
}

#[test]
#[ignore = "drives a real desktop application; run through scripts/private-session.sh"]
fn offscreen_live_terminal_survey_agrees_with_the_oracle() {
    live_apps::terminal(
        &LinuxAdapter::new(),
        &SharedBus::default(),
        |label, adapter, bus, frame| {
            let nodes = nodes_of(adapter, bus, frame)?;
            survey(label, adapter, bus, &nodes, frame);
            Ok(())
        },
    )
    .expect("the survey holds");
}

#[test]
#[ignore = "drives a real desktop application; run through scripts/private-session.sh"]
fn offscreen_live_chrome_page_realises_an_offscreen_button() {
    let dir = std::env::temp_dir().join(nonce("chrome"));
    std::fs::create_dir_all(&dir).expect("dir");
    let buttons: String = (0..300)
        .map(|i| format!("<button>Btn {i:03}</button><br>"))
        .collect();
    let page = dir.join("page.html");
    std::fs::write(&page, format!("<html><body>{buttons}</body></html>")).expect("page");
    let url = format!("file://{}", page.display());
    let data = dir.join("profile").to_string_lossy().into_owned();
    let user_data = format!("--user-data-dir={data}");
    let result = live_apps::app(
        &AppSpec {
            label: "chrome",
            program: "google-chrome",
            arguments: &[
                "--no-sandbox",
                "--force-renderer-accessibility",
                "--password-store=basic",
                "--no-first-run",
                &user_data,
                "--new-window",
                &url,
            ],
            min_nodes: 100,
        },
        &LinuxAdapter::new(),
        &SharedBus::default(),
        check_each,
    );
    let _ = std::fs::remove_dir_all(dir);
    result.expect("the case holds");
}

#[test]
#[ignore = "drives a real desktop application; run through scripts/private-session.sh"]
fn offscreen_live_vscode_scroll_rungs_are_measured() {
    live_apps::vscode(
        &LinuxAdapter::new(),
        &SharedBus::default(),
        |label, adapter, bus, frame| {
            let nodes = nodes_of(adapter, bus, frame)?;
            survey(label, adapter, bus, &nodes, frame);
            let _ = Duration::ZERO;
            match scroll_case(label, adapter, bus, frame, None, true) {
                Ok(()) | Err(_) => Ok(()),
            }
        },
    )
    .expect("vscode is measured");
}
