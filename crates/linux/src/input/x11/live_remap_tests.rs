use std::process::Command;
use std::time::{Duration, Instant};

use super::live_tests::{deadline, private_display};
use super::session::{live_keymap, run};
use crate::input::keymap_edit::KeymapEdit;
use crate::input::plan::Plan;
use crate::input::step::Step;

const MARKER: u32 = 0x1234;
const WRITTEN_LEVELS: usize = 4;

fn wait_for_keysyms(display: &str, code: u8, wanted: &[u32]) -> bool {
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(20) {
        let map = live_keymap(display, deadline(8_000)).expect("keymap");
        if map.keysyms_of(code).as_deref() == Some(wanted) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    false
}

#[test]
#[ignore = "needs scripts/private-session.sh --wm"]
fn live_a_remap_left_by_a_killed_client_is_restored_by_the_next_call() {
    let display = private_display();
    let map = live_keymap(&display, deadline(8_000)).expect("keymap");
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
        let _ = run(&display, vec![hold], &mut || Ok(()), deadline(70_000));
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
    let left = live_keymap(&display, deadline(8_000)).expect("keymap");
    assert_eq!(
        left.keysyms_of(code).as_deref(),
        Some(remapped.as_slice()),
        "the server restored the keymap by itself, so the journal is not needed"
    );
    let nudge = Plan {
        steps: vec![Step::MoveTo { x: 3, y: 3 }],
        release_origin: None,
    };
    run(&display, vec![nudge], &mut || Ok(()), deadline(8_000)).expect("next call");
    assert!(
        wait_for_keysyms(&display, code, &original),
        "the next call did not restore the keymap a killed client left remapped"
    );
}
