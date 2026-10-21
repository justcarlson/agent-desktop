use std::process::Command;
use std::time::{Duration, Instant};

use super::live_tests::{deadline, private_display};
use super::session::{live_keymap, run};
use crate::input::keymap_edit::KeymapEdit;
use crate::input::plan::Plan;
use crate::input::step::Step;

const MARKER: u32 = 0x1234;
const WRITTEN_LEVELS: usize = 4;

pub(crate) fn observed_keymap(display: &str) -> crate::input::keymap::Keymap {
    use super::{sink::X11Sink, xtest_sink::XtestSink};
    XtestSink::connect(display, deadline(8_000), Duration::from_secs(10))
        .unwrap()
        .keymap()
        .unwrap()
}

fn wait_for_keysyms(display: &str, code: u8, wanted: &[u32]) -> bool {
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(20) {
        let map = observed_keymap(display);
        if map.keysyms_of(code).as_deref() == Some(wanted) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    false
}

#[test]
#[ignore = "needs scripts/private-session.sh --wm"]
fn live_text_after_own_pending_remap_reaches_the_editor_and_bad_recovery_refuses() {
    use super::{held_journal::HeldJournal, sink::X11Sink, xtest_sink::XtestSink};
    use crate::input::{held_input::HeldInput, text_plan::plan_text};
    let display = private_display();
    let runtime = std::env::var("AGENT_DESKTOP_PRIVATE_SESSION").unwrap();
    let log = std::path::Path::new(&runtime).join("typed.txt");
    std::fs::write(&log, "").unwrap();
    let mut app = Command::new("python3")
        .args(["-c", super::live_tests::GTK3])
        .env("AD_LIVE_LOG", &log)
        .spawn()
        .unwrap();
    let (x, y) = super::live_tests::window_center("adx");
    let lease = crate::system::interaction_lease::acquire(deadline(30_000)).unwrap();
    run(
        &display,
        vec![Plan {
            steps: vec![
                Step::MoveTo { x, y },
                Step::Button {
                    code: 1,
                    down: true,
                },
                Step::Button {
                    code: 1,
                    down: false,
                },
            ],
            release_origin: None,
        }],
        &mut || Ok(()),
        &lease,
    )
    .unwrap();
    let map = live_keymap(&display, &lease).unwrap();
    let code = map.spare_keycodes(1)[0];
    let original = map.keysyms_of(code).unwrap();
    let journal = HeldJournal::in_runtime_dir(&display).unwrap();
    journal
        .record(
            &HeldInput::default(),
            &[KeymapEdit {
                keycode: code,
                keysyms: original.clone(),
            }],
        )
        .unwrap();
    let mut sink = XtestSink::connect(&display, deadline(8_000), Duration::from_secs(30)).unwrap();
    sink.edit_keymap(&KeymapEdit {
        keycode: code,
        keysyms: vec![0x01004f60; original.len()],
    })
    .unwrap();
    sink.round_trip().unwrap();
    let recovered = live_keymap(&display, &lease).unwrap();
    let plans = plan_text(&recovered, "你").unwrap();
    run(&display, plans, &mut || Ok(()), &lease).unwrap();
    let started = Instant::now();
    while std::fs::read_to_string(&log).unwrap() != "你" {
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "editor text: {:?}",
            std::fs::read_to_string(&log)
        );
        std::thread::sleep(Duration::from_millis(50));
    }
    let independent = Command::new("xmodmap").arg("-pke").output().unwrap();
    assert!(independent.status.success());
    let row = String::from_utf8(independent.stdout)
        .unwrap()
        .lines()
        .find(|row| row.split_whitespace().nth(1) == Some(&code.to_string()))
        .unwrap()
        .to_owned();
    assert!(row.split_once('=').unwrap().1.trim().is_empty(), "{row}");
    let record = std::path::Path::new(&runtime).join("agent-desktop-input/4000000000.json");
    std::fs::write(&record, "unreadable").unwrap();
    let refused = live_keymap(&display, &lease).unwrap_err();
    assert_eq!(
        refused.disposition,
        agent_desktop_core::DeliverySemantics::not_delivered()
    );
    assert_eq!(std::fs::read_to_string(&log).unwrap(), "你");
    app.kill().unwrap();
    app.wait().unwrap();
    std::fs::remove_file(log).unwrap();
}

#[test]
#[ignore = "needs scripts/private-session.sh --wm"]
fn live_a_remap_left_by_a_killed_client_is_restored_by_the_next_call() {
    let display = private_display();
    let map = observed_keymap(&display);
    let code = *map.spare_keycodes(1).first().expect("a spare keycode");
    let original = map.keysyms_of(code).expect("the row");
    let written = vec![MARKER; WRITTEN_LEVELS];
    let mut remapped = written.clone();
    remapped.resize(original.len(), 0);
    if std::env::var("AD_REMAP_HELPER").is_ok() {
        let hold = Plan {
            steps: vec![
                Step::Keymap(KeymapEdit {
                    keycode: code,
                    keysyms: written,
                }),
                Step::Pause(Duration::from_secs(60)),
            ],
            release_origin: None,
        };
        let lease = crate::system::interaction_lease::acquire(deadline(70_000)).expect("lease");
        let _ = run(&display, vec![hold], &mut || Ok(()), &lease);
        return;
    }
    let mut child = Command::new(std::env::current_exe().expect("exe"))
        .args([
            "--exact",
            "input::x11::live_remap_tests::live_a_remap_left_by_a_killed_client_is_restored_by_the_next_call",
            "--include-ignored",
            "--nocapture",
        ])
        .env("AD_REMAP_HELPER", "1")
        .spawn()
        .expect("helper");
    assert!(
        wait_for_keysyms(&display, code, &remapped),
        "the remap never appeared"
    );
    Command::new("kill")
        .args(["-KILL", &child.id().to_string()])
        .status()
        .expect("kill");
    let _ = child.wait();
    std::thread::sleep(Duration::from_millis(500));
    let left = observed_keymap(&display);
    assert_eq!(
        left.keysyms_of(code).as_deref(),
        Some(remapped.as_slice()),
        "the server restored the keymap by itself, so the journal is not needed"
    );
    let nudge = Plan {
        steps: vec![Step::MoveTo { x: 3, y: 3 }],
        release_origin: None,
    };
    let lease = crate::system::interaction_lease::acquire(deadline(8_000)).expect("lease");
    run(&display, vec![nudge], &mut || Ok(()), &lease).expect("next call");
    assert!(
        wait_for_keysyms(&display, code, &original),
        "the next call did not restore the keymap a killed client left remapped"
    );
}
