use std::process::Command;
use std::time::{Duration, Instant};

use agent_desktop_core::{Deadline, ErrorCode, WindowFilter, WindowInfo};

use super::{SharedBus, apps_scoped, list_windows, resolve_window_strict};
use super::{app_inventory, focused_window, window_id};
use crate::tree::live_launch::Launched;

const SETTLE: Duration = Duration::from_secs(40);
const CALCULATOR: &str = "gnome-calculator";

fn deadline(ms: u64) -> Deadline {
    Deadline::after(ms).expect("a deadline builds")
}

fn windows_of(bus: &SharedBus, app: &str) -> Vec<WindowInfo> {
    let filter = WindowFilter {
        focused_only: false,
        app: Some(app.to_owned()),
    };
    list_windows(bus, &filter, deadline(10_000)).unwrap_or_default()
}

fn wait_for_window(bus: &SharedBus, app: &str, pid: u32) -> WindowInfo {
    let started = Instant::now();
    while started.elapsed() < SETTLE {
        if let Some(window) = windows_of(bus, app)
            .into_iter()
            .find(|window| window.pid == pid)
        {
            return window;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    panic!("{app} (pid {pid}) never listed a window");
}

fn signal(name: &str, pid: u32) {
    let _ = Command::new("kill").args([name, &pid.to_string()]).status();
}

#[test]
#[ignore = "drives real desktop applications; run through scripts/remote-linux.sh"]
fn registry_live_lists_apps_and_windows_and_keeps_a_stopped_app() {
    let bus = SharedBus::default();
    let mut launched = Launched::new();
    let pid = launched.spawn(CALCULATOR, &[]);
    let window = wait_for_window(&bus, CALCULATOR, pid);
    window_id::decode(&window.id).expect("a listed id decodes");
    assert_eq!(window.app, CALCULATOR);
    let instance = window.process_instance.clone().expect("an instance");
    assert!(!instance.is_empty());

    let inventory = app_inventory(&bus, deadline(10_000)).expect("inventory");
    let app = inventory
        .apps
        .iter()
        .find(|app| app.pid == pid)
        .expect("calculator is an app");
    assert_eq!(app.process_instance.as_deref(), Some(instance.as_str()));
    println!(
        "registry_live apps={} skipped={} calculator={:?}",
        inventory.apps.len(),
        inventory.skipped.len(),
        app.presentation
    );

    let matched = apps_scoped(&bus, "GNOME-Calculator", None, deadline(10_000)).expect("scoped");
    assert_eq!(matched.len(), 1);
    assert!(
        apps_scoped(&bus, "gnome-calc", None, deadline(10_000))
            .expect("scoped")
            .is_empty()
    );

    let focused = list_windows(
        &bus,
        &WindowFilter {
            focused_only: true,
            app: None,
        },
        deadline(10_000),
    )
    .expect("focused listing");
    assert!(focused.len() <= 1, "at most one window is focused");
    println!(
        "registry_live focused={:?}",
        focused.iter().map(|w| &w.app).collect::<Vec<_>>()
    );
    println!(
        "registry_live focused_window={:?}",
        focused_window(&bus, deadline(10_000))
            .expect("focus")
            .map(|w| w.app)
    );

    let resolved = resolve_window_strict(&bus, &window, deadline(10_000)).expect("resolves");
    assert_eq!(resolved.id, window.id);
    let mut forged = window.clone();
    forged.process_instance = Some("forged:0".to_owned());
    assert_eq!(
        resolve_window_strict(&bus, &forged, deadline(10_000))
            .expect_err("forged")
            .code,
        ErrorCode::WindowNotFound
    );

    signal("-STOP", pid);
    let started = Instant::now();
    let stopped = app_inventory(&bus, deadline(10_000));
    let elapsed = started.elapsed();
    signal("-CONT", pid);
    let stopped = stopped.expect("a stopped app does not fail the inventory");
    let kept = stopped.apps.iter().find(|app| app.pid == pid);
    println!(
        "registry_live stopped: elapsed={}ms listed={} name={:?} skipped={}",
        elapsed.as_millis(),
        kept.is_some(),
        kept.map(|app| &app.name),
        stopped.skipped.len()
    );
    let kept = kept.expect("the stopped app is still listed");
    assert_eq!(kept.name, CALCULATOR);
    assert_eq!(kept.process_instance.as_deref(), Some(instance.as_str()));
    assert!(elapsed < Duration::from_secs(4));
}

#[test]
#[ignore = "drives real desktop applications; run through scripts/remote-linux.sh"]
fn registry_live_names_a_running_process_that_is_not_on_the_bus() {
    let bus = SharedBus::default();
    let mut launched = Launched::new();
    let name = format!("adnobus{}", std::process::id());
    let dir = std::env::temp_dir().join(&name);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    launched.scratch.push(dir.clone());
    let program = dir.join(&name);
    std::fs::copy("/bin/sleep", &program).expect("copy sleep");
    let pid = launched.spawn(&program.to_string_lossy(), &["60"]);
    let error = apps_scoped(&bus, &name, None, deadline(10_000)).expect_err("diagnostic");
    assert_eq!(error.code, ErrorCode::AppNotFound);
    assert!(error.message.contains("accessibility bus"));
    assert_eq!(
        error.details.as_ref().map(|details| &details["pid"]),
        Some(&serde_json::json!(pid))
    );
    assert!(
        apps_scoped(&bus, "no-such-process-anywhere", None, deadline(10_000))
            .expect("not an error")
            .is_empty()
    );
}

#[test]
#[ignore = "drives real desktop applications under a window manager; run through scripts/remote-linux.sh"]
fn registry_live_focus_agrees_with_the_window_manager() {
    use super::x11::{DisplayReader, X11Source, X11State};

    let home = std::env::var("HOME").expect("HOME");
    let root = format!("{home}/ad-wt/wm-pkgs/root");
    let bus = SharedBus::default();
    let mut launched = Launched::new();
    launched.spawn_with_env(
        &format!("{root}/usr/bin/openbox"),
        &["--config-file", &format!("{root}/etc/xdg/openbox/rc.xml")],
        &[
            (
                "LD_LIBRARY_PATH",
                format!("{root}/usr/lib/x86_64-linux-gnu"),
            ),
            ("XDG_DATA_DIRS", format!("{root}/usr/share:/usr/share")),
        ],
    );
    let calculator = launched.spawn(CALCULATOR, &[]);
    wait_for_window(&bus, CALCULATOR, calculator);
    let editor = launched.spawn("gnome-text-editor", &[]);
    wait_for_window(&bus, "gnome-text-editor", editor);

    let started = Instant::now();
    loop {
        let state = DisplayReader.read();
        let focused = focused_window(&bus, deadline(10_000)).expect("focus");
        let active_pid = match &state {
            X11State::Ready {
                active: Some(active),
                clients,
            } => clients
                .iter()
                .find(|client| client.id == *active)
                .and_then(|client| client.pid),
            _ => None,
        };
        println!(
            "registry_live x11 active_pid={active_pid:?} focused={:?}",
            focused.as_ref().map(|w| (&w.app, w.pid))
        );
        if let (Some(pid), Some(window)) = (active_pid, focused.as_ref())
            && window.pid == pid
        {
            let all = list_windows(&bus, &WindowFilter::default(), deadline(10_000)).expect("list");
            assert_eq!(
                all.iter().filter(|window| window.state.is_focused).count(),
                1
            );
            return;
        }
        assert!(
            started.elapsed() < SETTLE,
            "focus never agreed with the window manager"
        );
        std::thread::sleep(Duration::from_millis(300));
    }
}
