use std::time::{Duration, Instant};

use agent_desktop_core::{SystemOps, WindowFilter};

use super::list_windows;
use super::signal_live_apps::{
    Rig, capture, close_newest_frame, count_of, deadline, diff, found_event, identity_of, outcome,
    poll, press, process_gone, signal, tokens, wait_event, window_ids, x11_active_pid, x11_windows,
};

const OPEN_MS: u64 = 15_000;

fn editor(rig: &mut Rig) -> (u32, String) {
    let pid = rig.spawn("gnome-text-editor", &["--new-window"]);
    poll("the editor window", || {
        (x11_windows(pid).len() == 1).then_some(())
    });
    let (name, _) = identity_of(rig, pid);
    (pid, name)
}

fn calculator(rig: &mut Rig) -> (u32, String) {
    let pid = rig.spawn("gnome-calculator", &[]);
    poll("the calculator window", || {
        (x11_windows(pid).len() == 1).then_some(())
    });
    let (name, _) = identity_of(rig, pid);
    (pid, name)
}

fn pid_is(event: &serde_json::Value, pid: u32) -> bool {
    event["pid"].as_u64() == Some(u64::from(pid))
}

#[test]
#[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]
fn signal_live_text_editor_new_window_handoff_is_window_opened_without_app_launched() {
    let mut rig = Rig::new();
    let (pid, _) = editor(&mut rig);
    let seed = capture(&rig, None, None, 10_000).expect("the seed capture");
    let second = rig.spawn("gnome-text-editor", &["--new-window"]);
    poll("the second editor frame", || {
        (x11_windows(pid).len() == 2).then_some(())
    });
    poll("the handoff launcher to exit", || {
        process_gone(second).then_some(())
    });
    let result = wait_event(&rig, "window-opened", None, OPEN_MS, Some(Ok(seed.clone())));
    assert!(
        pid_is(found_event(&result), pid),
        "the new frame belongs to the old pid {pid}: {}",
        outcome(&result)
    );
    let after = capture(&rig, None, None, 10_000).expect("the capture after");
    let events = diff(&seed, &after);
    assert_eq!(
        count_of(&events, "window-opened"),
        1,
        "{:?}",
        tokens(&events)
    );
    assert_eq!(
        count_of(&events, "app-launched"),
        0,
        "{:?}",
        tokens(&events)
    );
    assert_eq!(after.windows.iter().filter(|w| w.pid == pid).count(), 2);
}

#[test]
#[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]
fn signal_live_calculator_launch_and_close_are_app_and_window_events() {
    let mut rig = Rig::new();
    let seed = capture(&rig, None, None, 10_000).expect("the seed capture");
    let pid = rig.spawn("gnome-calculator", &[]);
    let result = wait_event(&rig, "app-launched", None, OPEN_MS, Some(Ok(seed.clone())));
    assert!(pid_is(found_event(&result), pid), "{}", outcome(&result));
    poll("the calculator window", || {
        (x11_windows(pid).len() == 1).then_some(())
    });
    let (name, _) = identity_of(&rig, pid);
    let opened = capture(&rig, None, None, 10_000).expect("capture");
    let events = diff(&seed, &opened);
    assert_eq!(
        count_of(&events, "app-launched"),
        1,
        "{:?}",
        tokens(&events)
    );
    assert_eq!(
        count_of(&events, "window-opened"),
        1,
        "{:?}",
        tokens(&events)
    );

    press(&rig, &name, "ctrl+q").expect("Ctrl+Q reaches the calculator");
    poll("the calculator to exit", || process_gone(pid).then_some(()));
    let result = wait_event(
        &rig,
        "app-terminated",
        None,
        OPEN_MS,
        Some(Ok(opened.clone())),
    );
    assert!(pid_is(found_event(&result), pid), "{}", outcome(&result));
    let closed = capture(&rig, None, None, 10_000).expect("capture");
    let events = diff(&opened, &closed);
    assert_eq!(
        count_of(&events, "window-closed"),
        1,
        "{:?}",
        tokens(&events)
    );
    assert_eq!(
        count_of(&events, "app-terminated"),
        1,
        "{:?}",
        tokens(&events)
    );
    assert!(x11_windows(pid).is_empty());
}

#[test]
#[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]
fn signal_live_second_calculator_process_is_a_new_app_with_a_distinct_id() {
    let mut rig = Rig::new();
    let (first, _) = calculator(&mut rig);
    let seed = capture(&rig, None, None, 10_000).expect("the seed capture");
    let second = rig.spawn("gnome-calculator", &[]);
    assert_ne!(first, second);
    poll("the second calculator window", || {
        (x11_windows(second).len() == 1).then_some(())
    });
    let result = wait_event(&rig, "app-launched", None, OPEN_MS, Some(Ok(seed.clone())));
    assert!(pid_is(found_event(&result), second), "{}", outcome(&result));
    let after = capture(&rig, None, None, 10_000).expect("capture");
    let events = diff(&seed, &after);
    assert_eq!(
        count_of(&events, "window-opened"),
        1,
        "{:?}",
        tokens(&events)
    );
    let window_of = |pid| {
        after
            .windows
            .iter()
            .find(|w| w.pid == pid)
            .expect("a window")
    };
    let (one, two) = (window_of(first), window_of(second));
    assert_ne!(one.id, two.id, "the two processes list distinct window ids");
    assert_eq!(one.title, two.title, "both calculators carry one title");
}

#[test]
#[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]
fn signal_live_text_editor_ctrl_n_is_window_opened_under_the_same_pid() {
    let mut rig = Rig::new();
    let (pid, name) = editor(&mut rig);
    let seed = capture(&rig, None, None, 10_000).expect("the seed capture");
    press(&rig, &name, "ctrl+n").expect("Ctrl+N reaches the editor");
    poll("the second editor window", || {
        (x11_windows(pid).len() == 2).then_some(())
    });
    let result = wait_event(
        &rig,
        "window-opened",
        Some(&name),
        OPEN_MS,
        Some(Ok(seed.clone())),
    );
    assert!(pid_is(found_event(&result), pid), "{}", outcome(&result));
    let after = capture(&rig, None, None, 10_000).expect("capture");
    let events = diff(&seed, &after);
    assert_eq!(
        count_of(&events, "app-launched"),
        0,
        "{:?}",
        tokens(&events)
    );
    assert_eq!(
        count_of(&events, "window-opened"),
        1,
        "{:?}",
        tokens(&events)
    );
}

#[test]
#[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]
fn signal_live_sigkilled_calculator_is_window_closed_and_app_terminated() {
    let mut rig = Rig::new();
    let (pid, _) = calculator(&mut rig);
    let seed = capture(&rig, None, None, 10_000).expect("the seed capture");
    signal("-KILL", pid);
    let killed = Instant::now();
    let result = wait_event(
        &rig,
        "app-terminated",
        None,
        OPEN_MS,
        Some(Ok(seed.clone())),
    );
    let seen_after = killed.elapsed();
    assert!(pid_is(found_event(&result), pid), "{}", outcome(&result));
    println!(
        "signal_live sigkill: app-terminated seen {}ms after the kill",
        seen_after.as_millis()
    );
    assert!(seen_after < Duration::from_secs(5));
    assert!(process_gone(pid));
    let after = capture(&rig, None, None, 10_000).expect("capture");
    let events = diff(&seed, &after);
    assert_eq!(
        count_of(&events, "window-closed"),
        1,
        "{:?}",
        tokens(&events)
    );
    assert_eq!(
        count_of(&events, "app-terminated"),
        1,
        "{:?}",
        tokens(&events)
    );
}

#[test]
#[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]
fn signal_live_two_windows_focus_changes_under_the_window_manager() {
    let mut rig = Rig::new();
    let (editor_pid, _) = editor(&mut rig);
    let (calculator_pid, _) = calculator(&mut rig);
    poll("the calculator to take focus", || {
        (x11_active_pid() == Some(calculator_pid)).then_some(())
    });
    let filter = WindowFilter {
        focused_only: false,
        app: None,
    };
    let windows = list_windows(rig.bus(), &filter, deadline(10_000)).expect("windows");
    let target = windows
        .iter()
        .find(|w| w.pid == editor_pid)
        .expect("the editor window is listed");
    let seed = capture(&rig, None, None, 10_000).expect("the seed capture");
    let lease = rig
        .adapter
        .acquire_interaction_lease(deadline(10_000))
        .expect("a lease");
    rig.adapter.focus_window(target, &lease).expect("focus");
    poll("the editor to take focus", || {
        (x11_active_pid() == Some(editor_pid)).then_some(())
    });
    let result = wait_event(&rig, "focus-changed", None, OPEN_MS, Some(Ok(seed)));
    assert!(
        pid_is(found_event(&result), editor_pid),
        "{}",
        outcome(&result)
    );
}

#[test]
#[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]
fn signal_live_window_that_opens_and_closes_inside_one_wait_is_reported() {
    let mut rig = Rig::new();
    let (pid, name) = editor(&mut rig);
    let seed = capture(&rig, None, None, 10_000).expect("the seed capture");
    let known = window_ids(&rig, &name);
    let result = std::thread::scope(|scope| {
        let waiter = scope.spawn(|| {
            wait_event(
                &rig,
                "window-closed",
                Some(&name),
                40_000,
                Some(Ok(seed.clone())),
            )
        });
        press(&rig, &name, "ctrl+n").expect("Ctrl+N");
        let fresh = poll("the new window id", || {
            let ids = window_ids(&rig, &name);
            ids.into_iter().find(|id| !known.contains(id))
        });
        std::thread::sleep(Duration::from_millis(2_000));
        close_newest_frame(&rig, pid);
        poll("the second editor window to close", || {
            (x11_windows(pid).len() == 1).then_some(())
        });
        (waiter.join().expect("the waiter finished"), fresh)
    });
    let (result, fresh) = result;
    let event = found_event(&result);
    assert_eq!(
        event["window_id"], fresh,
        "the window that opened and closed"
    );
    assert!(seed.windows.iter().all(|w| w.id != fresh));
}

#[test]
#[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]
fn signal_live_batch_seed_before_the_action_is_complete_and_the_close_is_found() {
    let mut rig = Rig::new();
    let (pid, name) = editor(&mut rig);
    let seed = capture(&rig, Some(&name), None, 10_000).expect("a scoped seed does not abort");
    assert!(
        seed.completeness.windows && seed.completeness.apps && seed.completeness.surfaces,
        "{:?}",
        seed.completeness
    );
    let known = window_ids(&rig, &name);
    press(&rig, &name, "ctrl+n").expect("Ctrl+N");
    poll("the second editor window", || {
        (x11_windows(pid).len() == 2).then_some(())
    });
    let result = wait_event(&rig, "window-opened", Some(&name), OPEN_MS, Some(Ok(seed)));
    found_event(&result);

    let seed = capture(&rig, Some(&name), None, 10_000).expect("the second seed");
    let fresh = window_ids(&rig, &name)
        .into_iter()
        .find(|id| !known.contains(id))
        .expect("the window ctrl+n opened");
    close_newest_frame(&rig, pid);
    let result = wait_event(&rig, "window-closed", Some(&name), OPEN_MS, Some(Ok(seed)));
    assert_eq!(found_event(&result)["window_id"], fresh);
    assert_eq!(x11_windows(pid).len(), 1);
}
