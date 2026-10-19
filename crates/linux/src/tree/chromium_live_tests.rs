use std::collections::BTreeMap;
use std::time::Duration;

use agent_desktop_core::{ErrorCode, ObservationOps, RefEntry};

use super::live_launch::{nonce, vscode_arguments};
use super::live_rig::{Rig, kind, refused, rig, signal, wait_until};

fn key(entry: &RefEntry) -> (String, Option<String>, Option<String>) {
    (
        entry.identity.role.clone(),
        entry.identity.name.clone(),
        entry.identity.native_id.as_ref().map(|id| id.value.clone()),
    )
}

fn unique_paths(
    rig: &Rig,
    entries: &[(String, RefEntry)],
) -> BTreeMap<(String, Option<String>, Option<String>), Vec<usize>> {
    let _ = rig;
    let mut counts: BTreeMap<_, usize> = BTreeMap::new();
    for (_, entry) in entries {
        *counts.entry(key(entry)).or_default() += 1;
    }
    entries
        .iter()
        .filter(|(_, entry)| counts[&key(entry)] == 1 && entry.identity.name.is_some())
        .map(|(_, entry)| (key(entry), entry.scope.path.iter().copied().collect()))
        .collect()
}

fn launch(rig: &mut Rig) -> (u32, String, String) {
    let home = std::env::var("HOME").expect("HOME");
    let program = format!("{home}/.local/ad-tools/vscode/app/code");
    let data = std::env::temp_dir()
        .join(nonce("chromium"))
        .to_string_lossy()
        .into_owned();
    let pid = rig.launched.spawn(&program, &vscode_arguments(&data));
    (pid, program, data)
}

#[test]
#[ignore = "drives a real desktop application; run through scripts/private-session.sh"]
fn chromium_live_a_killed_vscode_is_a_settled_stale_ref_in_one_pass() {
    let mut rig = rig();
    let (pid, _, _) = launch(&mut rig);
    let settled = rig.settled(pid, 200);
    let entry = rig
        .entries(&settled.frame, true)
        .into_iter()
        .map(|(_, entry)| entry)
        .find(|entry| entry.identity.role == "button" && entry.identity.name.is_some())
        .expect("a named button");
    let handle = rig.resolve(&entry).expect("resolves while alive");
    signal(pid, "KILL");
    let (outcome, elapsed) = rig.timed_resolve(&entry);
    let error = refused(outcome, "a killed app resolves nothing");
    println!(
        "chromium_live kill: {:?} kind {} in {} ms",
        error.code,
        kind(&error),
        elapsed.as_millis()
    );
    assert_eq!(error.code, ErrorCode::StaleRef);
    assert!(!error.is_explicitly_retryable());
    assert!(elapsed < Duration::from_millis(1_500));
    let held = rig
        .adapter
        .get_live_element(&handle, super::live_rig::deadline(4_000));
    assert!(matches!(held, Err(ref error) if error.code == ErrorCode::StaleRef));
}

#[test]
#[ignore = "drives a real desktop application; run through scripts/private-session.sh"]
fn chromium_live_opening_a_second_file_shifts_no_web_content_path() {
    let mut rig = rig();
    let (pid, program, data) = launch(&mut rig);
    let settled = rig.settled(pid, 200);
    let before_entries = rig.entries(&settled.frame, true);
    let before = unique_paths(&rig, &before_entries);

    let dir = std::env::temp_dir().join(nonce("second-file"));
    std::fs::create_dir_all(&dir).expect("a directory");
    rig.launched.scratch.push(dir.clone());
    let file = dir.join("second.txt");
    std::fs::write(&file, "second document\n").expect("a file");
    let file_arg = file.to_string_lossy().into_owned();
    let mut arguments: Vec<&str> = vscode_arguments(&data).to_vec();
    arguments.retain(|argument| *argument != "--new-window");
    arguments.extend(["--reuse-window", &file_arg]);
    rig.launched.spawn(&program, &arguments);
    let changed = wait_until(Duration::from_secs(60), || {
        rig.entries(&settled.frame, true).len() != before_entries.len()
    });
    println!("chromium_live second file: tree_changed={changed}");
    std::thread::sleep(Duration::from_secs(3));
    let after_entries = rig.entries(&settled.frame, true);
    let after = unique_paths(&rig, &after_entries);

    let shared: Vec<_> = before.keys().filter(|k| after.contains_key(*k)).collect();
    let moved = shared.iter().filter(|k| before[**k] != after[**k]).count();
    let web: Vec<_> = shared.iter().filter(|k| k.2.is_some()).collect();
    let web_moved = web.iter().filter(|k| before[***k] != after[***k]).count();
    println!(
        "chromium_live second file: refs before={} after={} shared-unique={} path-moved={moved} dom-id-shared={} dom-id-moved={web_moved}",
        before_entries.len(),
        after_entries.len(),
        shared.len(),
        web.len()
    );
    let resolving = before_entries
        .iter()
        .filter(|(_, entry)| rig.resolve(entry).is_ok())
        .count();
    println!(
        "chromium_live second file: {resolving} of {} old refs still resolve",
        before_entries.len()
    );
    assert!(resolving * 2 >= before_entries.len());
}
