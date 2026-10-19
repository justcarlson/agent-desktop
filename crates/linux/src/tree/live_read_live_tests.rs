use std::collections::BTreeSet;
use std::time::Duration;

use agent_desktop_core::{ObservationOps, Rect, RefEntry, state};

use super::live_apps::each_app;
use super::live_fixture::{self, Fixture};
use super::live_launch::Launched;
use super::live_rig::{Rig, deadline, rig, wait_until};
use super::{AtspiElement, Field};
use crate::LinuxAdapter;
use crate::system::SharedBus;

const VOLATILE: [&str; 3] = [state::FOCUSED, state::SELECTED, state::PRESSED];

fn stable(tokens: &[String]) -> BTreeSet<String> {
    tokens
        .iter()
        .filter(|token| !VOLATILE.contains(&token.as_str()) && *token != state::HIDDEN)
        .cloned()
        .collect()
}

fn expected_bounds(rig: &Rig, handle: &agent_desktop_core::NativeHandle) -> Option<Rect> {
    match rig.raw(handle).ok()?.extents {
        Field::Present(extents) if extents.x != i32::MIN && extents.y != i32::MIN => {
            Some(Rect::from(extents))
        }
        _ => None,
    }
}

fn read_check(
    label: &str,
    adapter: &LinuxAdapter,
    bus: &SharedBus,
    frame: &AtspiElement,
) -> Result<(), String> {
    let _ = bus;
    let rig = rig();
    let entries = rig.entries(frame, true);
    let (mut checked, mut sentinel, mut problems) = (0usize, 0usize, Vec::new());
    for (id, entry) in &entries {
        let Ok(handle) = adapter.resolve_element_strict(entry, deadline(5_000)) else {
            continue;
        };
        let live = adapter
            .get_live_element(&handle, deadline(5_000))
            .map_err(|error| {
                format!("{id} get_live_element: {:?} {}", error.code, error.message)
            })?;
        let mut differences = Vec::new();
        if live.state.role != entry.identity.role {
            differences.push(format!(
                "role {} against {}",
                live.state.role, entry.identity.role
            ));
        }
        if let (Some(name), Some(stored)) = (live.identity.name.known(), &entry.identity.name)
            && name != stored
        {
            differences.push(format!("name {name:?} against {stored:?}"));
        }
        let second = expected_bounds(&rig, &handle);
        if live.bounds != second {
            differences.push(format!("bounds {:?} against {second:?}", live.bounds));
        }
        if live.bounds.is_none() {
            sentinel += 1;
            if adapter
                .get_element_bounds(&handle, deadline(5_000))
                .ok()
                .flatten()
                .is_some()
            {
                differences.push("get_element_bounds disagrees with get_live_element".to_owned());
            }
        }
        if stable(&live.state.states) != stable(&entry.capabilities.states) {
            differences.push(format!(
                "states {:?} against {:?}",
                live.state.states, entry.capabilities.states
            ));
        }
        if live.available_actions != entry.capabilities.available_actions {
            differences.push(format!(
                "actions {:?} against {:?}",
                live.available_actions, entry.capabilities.available_actions
            ));
        }
        if !differences.is_empty() {
            problems.push(format!(
                "{id} {} {:?}: {}",
                entry.identity.role,
                entry.identity.name,
                differences.join("; ")
            ));
        }
        checked += 1;
    }
    println!(
        "live_read {label}: refs={} read={checked} without_bounds={sentinel} problems={}",
        entries.len(),
        problems.len()
    );
    if checked == 0 {
        problems.push("no ref could be read".to_owned());
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("\n"))
    }
}

#[test]
#[ignore = "drives real desktop applications; run through scripts/private-session.sh"]
fn live_read_every_projection_agrees_with_a_second_read_on_each_toolkit() {
    let adapter = LinuxAdapter::new();
    let bus = SharedBus::default();
    let failures: Vec<String> = each_app(&adapter, &bus, read_check)
        .into_iter()
        .filter_map(|(label, outcome)| outcome.err().map(|error| format!("[{label}] {error}")))
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

fn named_entry(entries: &[(String, RefEntry)], role: &str, name: Option<&str>) -> RefEntry {
    entries
        .iter()
        .map(|(_, entry)| entry)
        .find(|entry| entry.identity.role == role && entry.identity.name.as_deref() == name)
        .unwrap_or_else(|| panic!("no {role} named {name:?}"))
        .clone()
}

#[test]
#[ignore = "drives a real desktop application; run through scripts/private-session.sh"]
fn live_read_a_calculator_display_follows_a_button_press_made_by_the_harness() {
    let mut rig = rig();
    let pid = rig.launched.spawn("gnome-calculator", &[]);
    let settled = rig.settled(pid, 30);
    let entries = rig.entries(&settled.frame, true);
    let display = named_entry(&entries, "textfield", None);
    let seven = named_entry(&entries, "button", Some("7"));
    let eight = named_entry(&entries, "button", Some("8"));
    let shown = || {
        let handle = rig.resolve(&display).expect("the display resolves");
        rig.adapter
            .get_live_value(&handle, deadline(5_000))
            .expect("the display reads")
    };
    let before = shown();
    for button in [&seven, &eight] {
        let handle = rig.resolve(button).expect("the button resolves");
        rig.do_action(&handle, 0).expect("the button is pressed");
    }
    assert!(
        wait_until(Duration::from_secs(5), || shown().as_deref() == Some("78")),
        "the display read {:?} (it read {before:?} before)",
        shown()
    );
}

fn secure_values(rig: &Rig, frame: &AtspiElement, label: &str, exposed_expected: usize) {
    let mut secure = 0;
    let mut plain = 0;
    let mut exposed = 0;
    for (id, entry) in rig.entries(frame, true) {
        if entry.identity.role != "textfield" {
            continue;
        }
        let Ok(handle) = rig.resolve(&entry) else {
            continue;
        };
        let state = rig
            .adapter
            .get_live_state(&handle, deadline(5_000))
            .expect("the state reads")
            .expect("a state");
        let value = rig
            .adapter
            .get_live_value(&handle, deadline(5_000))
            .expect("the value reads");
        println!(
            "live_read secure {label} {id}: states={:?} value={value:?}",
            state.states
        );
        if state.states.iter().any(|token| token == state::SECURE) {
            secure += 1;
            assert_eq!(value, None, "[{label}] {id} value is withheld");
            assert_eq!(state.value, None, "[{label}] {id} state value is withheld");
        } else if value.as_deref() == Some("plain-text") {
            plain += 1;
        } else if value.as_deref() == Some("hunter2") {
            exposed += 1;
        }
    }
    assert!(secure >= 1, "[{label}] a secure field was found");
    assert_eq!(plain, 1, "[{label}] the plain field still reads its value");
    assert_eq!(
        exposed, exposed_expected,
        "[{label}] fields that hide their text but publish no secure role"
    );
}

#[test]
#[ignore = "drives real desktop applications; run through scripts/private-session.sh"]
fn live_read_a_password_value_is_withheld_on_gtk3_gtk4_and_qt() {
    for (label, make, exposed) in [
        (
            "gtk3",
            live_fixture::gtk3 as fn(&mut Launched) -> Result<Fixture, String>,
            0,
        ),
        ("gtk4", live_fixture::gtk4, 1),
        ("qt", live_fixture::qt, 0),
    ] {
        let mut rig = rig();
        let fixture = make(&mut rig.launched).expect("the fixture starts");
        let settled = rig.settled(fixture.pid, 4);
        secure_values(&rig, &settled.frame, label, exposed);
    }
}
