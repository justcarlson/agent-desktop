use std::process::Command;
use std::time::{Duration, Instant};

use agent_desktop_core::commands::press::{PressArgs, execute as press_execute};
use agent_desktop_core::commands::wait::{
    WaitArgs, WaitModeArgs, WaitPredicateArgs, execute as wait_execute,
};
use agent_desktop_core::commands::wait_surface::SurfaceWait;
use agent_desktop_core::{
    AdapterError, AppError, CommandContext, Deadline, InteractionPolicy, ProcessIdentity,
    SignalBaseline, SignalFilter, SystemOps, UiEvent, WindowFilter, diff_signals,
};
use serde_json::Value;
use zbus::blocking::Connection;

use super::{SharedBus, app_inventory, list_windows};
use crate::LinuxAdapter;
use crate::tree::live_launch::Launched;

#[path = "signal_live_launch.rs"]
mod launch;
#[path = "signal_live_oracle.rs"]
mod oracle;

pub(super) use launch::{
    close_newest_frame, dismiss_dialog, launch_qt, main_menu_toggle, open_gtk4_menu_item,
    scratch_dir, terminal_server,
};
pub(super) use oracle::{
    FRAME, MENU, MENU_BUTTON, MENU_ITEM, Node, POPUP_MENU, STATE_CHECKED, STATE_SHOWING, TOGGLE,
    descendants, do_action, process_gone, process_state, showing_of, top_level, within,
    x11_active_pid, x11_windows,
};

pub(super) const SETTLE: Duration = Duration::from_secs(40);

pub(super) fn deadline(ms: u64) -> Deadline {
    Deadline::after(ms).expect("a deadline builds")
}

pub(super) struct Rig {
    pub(super) adapter: LinuxAdapter,
    pub(super) launched: Launched,
    pub(super) connection: Connection,
    adopted: Vec<u32>,
}

impl Rig {
    pub(super) fn new() -> Self {
        let launched = Launched::new();
        let adapter = LinuxAdapter::new();
        let connection = adapter
            .bus()
            .get(deadline(10_000))
            .expect("the accessibility bus of the private session");
        Self {
            adapter,
            launched,
            connection,
            adopted: Vec::new(),
        }
    }

    pub(super) fn bus(&self) -> &SharedBus {
        self.adapter.bus()
    }

    pub(super) fn spawn(&mut self, program: &str, args: &[&str]) -> u32 {
        require_program(program);
        self.launched.spawn(program, args)
    }

    pub(super) fn adopt(&mut self, pid: u32) -> u32 {
        self.adopted.push(pid);
        pid
    }
}

impl Drop for Rig {
    fn drop(&mut self) {
        for pid in self.adopted.drain(..) {
            signal("-CONT", pid);
            signal("-KILL", pid);
        }
    }
}

pub(super) fn require_program(program: &str) {
    let found = std::env::var_os("PATH")
        .into_iter()
        .flat_map(|paths| std::env::split_paths(&paths).collect::<Vec<_>>())
        .any(|dir| dir.join(program).is_file());
    assert!(
        found || std::path::Path::new(program).is_file(),
        "precondition: {program} is not installed on this box"
    );
}

pub(super) fn signal(name: &str, pid: u32) {
    let _ = Command::new("kill").args([name, &pid.to_string()]).status();
}

pub(super) struct Stopped(pub(super) u32);

impl Stopped {
    pub(super) fn new(pid: u32) -> Self {
        signal("-STOP", pid);
        Self(pid)
    }
}

impl Drop for Stopped {
    fn drop(&mut self) {
        signal("-CONT", self.0);
    }
}

pub(super) fn poll<T>(what: &str, mut attempt: impl FnMut() -> Option<T>) -> T {
    let started = Instant::now();
    while started.elapsed() < SETTLE {
        if let Some(value) = attempt() {
            return value;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    panic!("{what} did not happen within {}s", SETTLE.as_secs());
}

pub(super) fn identity_of(rig: &Rig, pid: u32) -> (String, ProcessIdentity) {
    let app = poll("the app listing", || {
        app_inventory(rig.bus(), deadline(10_000))
            .ok()?
            .apps
            .into_iter()
            .find(|app| app.pid == pid)
    });
    let instance = app.process_instance.expect("an instance");
    (app.name, ProcessIdentity::new(pid, instance))
}

pub(super) fn capture(
    rig: &Rig,
    app: Option<&str>,
    process: Option<&ProcessIdentity>,
    ms: u64,
) -> Result<SignalBaseline, AdapterError> {
    let filter = SignalFilter {
        app: app.map(str::to_owned),
        process: process.cloned(),
    };
    rig.adapter.capture_signal_baseline(&filter, deadline(ms))
}

pub(super) fn diff(before: &SignalBaseline, after: &SignalBaseline) -> Vec<UiEvent> {
    diff_signals(before, after)
}

pub(super) fn tokens(events: &[UiEvent]) -> Vec<&'static str> {
    events.iter().map(|event| event.kind.cli_token()).collect()
}

pub(super) fn count_of(events: &[UiEvent], token: &str) -> usize {
    tokens(events).iter().filter(|t| **t == token).count()
}

pub(super) fn wait_event(
    rig: &Rig,
    event: &str,
    app: Option<&str>,
    timeout_ms: u64,
    seed: Option<Result<SignalBaseline, AdapterError>>,
) -> Result<Value, AppError> {
    let mode = WaitModeArgs {
        event: Some(event.to_owned()),
        ..WaitModeArgs::default()
    };
    wait_with(rig, mode, app, timeout_ms, seed)
}

pub(super) fn wait_menu(
    rig: &Rig,
    app: &str,
    open: bool,
    timeout_ms: u64,
) -> Result<Value, AppError> {
    let surface = if open {
        SurfaceWait::Menu
    } else {
        SurfaceWait::MenuClosed
    };
    let mode = WaitModeArgs {
        surface: Some(surface),
        ..WaitModeArgs::default()
    };
    wait_with(rig, mode, Some(app), timeout_ms, None)
}

fn wait_with(
    rig: &Rig,
    mode: WaitModeArgs,
    app: Option<&str>,
    timeout_ms: u64,
    seed: Option<Result<SignalBaseline, AdapterError>>,
) -> Result<Value, AppError> {
    let args = WaitArgs {
        mode,
        predicate: WaitPredicateArgs {
            predicate: None,
            value: None,
            action: None,
            count: None,
        },
        timeout_ms,
        app: app.map(str::to_owned),
    };
    let context = CommandContext::default().with_event_baseline(seed);
    wait_execute(args, &rig.adapter, &context)
}

pub(super) fn press(rig: &Rig, app: &str, combo: &str) -> Result<Value, AppError> {
    press_with(rig, app, None, combo, CommandContext::default())
        .or_else(|_| press_headed(rig, app, None, combo))
}

pub(super) fn window_ids(rig: &Rig, app: &str) -> Vec<String> {
    let filter = WindowFilter {
        focused_only: false,
        app: Some(app.to_owned()),
    };
    list_windows(rig.bus(), &filter, deadline(10_000))
        .map(|windows| windows.into_iter().map(|w| w.id).collect())
        .unwrap_or_default()
}

pub(super) fn press_headed(
    rig: &Rig,
    app: &str,
    window_id: Option<&str>,
    combo: &str,
) -> Result<Value, AppError> {
    let headed = CommandContext::default().with_interaction_policy(InteractionPolicy::headed());
    press_with(rig, app, window_id, combo, headed)
}

fn press_with(
    rig: &Rig,
    app: &str,
    window_id: Option<&str>,
    combo: &str,
    context: CommandContext,
) -> Result<Value, AppError> {
    let args = PressArgs {
        combo: combo.to_owned(),
        app: Some(app.to_owned()),
        window_id: window_id.map(str::to_owned),
        force: true,
    };
    press_execute(args, &rig.adapter, &context)
}

pub(super) fn close_with_escape(rig: &Rig, app: &str, done: impl Fn() -> bool) {
    poll("the Escape key to close the surface", || {
        if done() {
            return Some(());
        }
        let _ = press(rig, app, "escape");
        std::thread::sleep(Duration::from_millis(400));
        done().then_some(())
    });
}

pub(super) fn details_of(error: &AppError) -> Value {
    match error {
        AppError::Adapter(adapter) => adapter.details.clone().unwrap_or(Value::Null),
        _ => Value::Null,
    }
}

pub(super) fn found_event(result: &Result<Value, AppError>) -> &Value {
    match result {
        Ok(value) => {
            assert_eq!(value["found"], true, "{}", outcome(result));
            &value["event"]
        }
        Err(error) => panic!("the wait failed: {}", error.code()),
    }
}

pub(super) fn outcome(result: &Result<Value, AppError>) -> String {
    match result {
        Ok(value) if value["found"] == true => "wait found its event".into(),
        Ok(_) => "wait ended without its event".into(),
        Err(error) => format!("wait failed with {}", error.code()),
    }
}
