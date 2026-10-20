use std::collections::HashSet;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::time::Duration;

use agent_desktop_core::launch_options::LaunchOptions;
use agent_desktop_core::launch_result::LaunchResult;
use agent_desktop_core::{
    AdapterError, Deadline, DeliverySemantics, ErrorCode, InteractionLease, ProcessId,
    RendererKind, WindowFilter, WindowInfo,
};

use super::app_alias::current_index;
use super::focus_window::focus_window;
use super::launch_attach::{Decision, Running, decide, identity_keys};
use super::launch_observe::{Found, POLL_INTERVAL, Spawned, find_owner, pids_of, running_of};
use super::launch_target::{Plan, Search, classify, plan};
use super::liveness::{Liveness, liveness};
use super::process_facts::{self, WebFamily};
use super::registry::read_registry;
use super::{SharedBus, list_windows};

fn before_spawn(error: AdapterError) -> AdapterError {
    error.with_disposition(DeliverySemantics::not_delivered())
}

fn invalid(message: &str) -> AdapterError {
    before_spawn(AdapterError::new(ErrorCode::InvalidArgs, message))
}

fn validate(options: &LaunchOptions) -> Result<(), AdapterError> {
    if options.args.iter().any(|arg| arg.contains('\0')) {
        return Err(invalid("A launch argument holds a NUL"));
    }
    let bad_env = options
        .env
        .iter()
        .any(|(key, value)| key.is_empty() || key.contains(['=', '\0']) || value.contains('\0'));
    if bad_env {
        return Err(invalid(
            "A launch environment entry has an invalid name or value",
        ));
    }
    if options.cwd.as_ref().is_some_and(|dir| !dir.is_dir()) {
        return Err(invalid("The launch working directory is not a directory"));
    }
    Ok(())
}

fn spawn_error(identifier: &str, error: &std::io::Error) -> AdapterError {
    let (code, message) = match error.kind() {
        std::io::ErrorKind::NotFound => (
            ErrorCode::AppNotFound,
            format!("'{identifier}' could not be started: the program was not found"),
        ),
        std::io::ErrorKind::PermissionDenied => (
            ErrorCode::ActionFailed,
            format!("'{identifier}' could not be started: the program is not executable"),
        ),
        kind => (
            ErrorCode::ActionFailed,
            format!("'{identifier}' could not be started ({kind})"),
        ),
    };
    let mut refusal = before_spawn(AdapterError::new(code, message));
    if let Some(errno) = error.raw_os_error() {
        refusal = refusal.with_platform_detail(format!("errno {errno}"));
    }
    refusal
}

fn spawn(identifier: &str, plan: &Plan, options: &LaunchOptions) -> Result<Spawned, AdapterError> {
    let (program, arguments) = plan
        .argv
        .split_first()
        .ok_or_else(|| invalid("The launch command is empty"))?;
    let mut command = Command::new(program);
    command
        .args(arguments)
        .envs(&options.env)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0);
    let directory = options
        .cwd
        .as_ref()
        .or(plan.cwd.as_ref().filter(|dir| dir.is_dir()));
    if let Some(directory) = directory {
        command.current_dir(directory);
    }
    command
        .spawn()
        .map(Spawned::new)
        .map_err(|error| spawn_error(identifier, &error))
}

fn bounded(deadline: Deadline, timeout_ms: u64) -> Deadline {
    if timeout_ms == 0 {
        deadline
    } else {
        deadline.capped(Duration::from_millis(timeout_ms))
    }
}

fn chromium_renderer(pid: u32) -> Option<RendererKind> {
    process_facts::read(pid)
        .and_then(|facts| facts.web_family)
        .filter(|family| *family == WebFamily::Chromium)
        .map(|_| RendererKind::Chromium)
}

/// The result for an owner and its window. A window is brought forward once,
/// only when asked, and a refusal is reported in the suggestion because the
/// launch itself succeeded.
fn conclude(
    owner: &Running,
    window: Option<WindowInfo>,
    options: &LaunchOptions,
    created: bool,
    focus: impl FnOnce(&WindowInfo) -> Result<(), AdapterError>,
) -> LaunchResult {
    let mut notes = Vec::new();
    if !created && (!options.env.is_empty() || options.cwd.is_some()) {
        notes.push(
            "A running instance was used, so the environment and working directory of this launch were not applied.".to_owned(),
        );
    }
    if options.activate
        && let Some(window) = &window
        && let Err(error) = focus(window)
    {
        notes.push(format!(
            "The window could not be brought forward: {}",
            error.message
        ));
    }
    LaunchResult {
        app: owner.name.clone(),
        pid: ProcessId::new(owner.pid),
        process_instance: Some(owner.instance.clone()),
        window,
        cdp: None,
        renderer: chromium_renderer(owner.pid),
        suggestion: (!notes.is_empty()).then(|| notes.join(" ")),
    }
}

/// The refusal for an application that was on the registry and left before it
/// showed a window: reporting its pid as a launch would hand the caller a dead
/// process.
fn exited_early(owner: &Running, liveness: Liveness) -> Result<(), AdapterError> {
    if liveness != Liveness::Gone {
        return Ok(());
    }
    Err(AdapterError::new(
        ErrorCode::ActionFailed,
        format!(
            "'{}' (pid {}) exited before it showed a window",
            owner.name, owner.pid
        ),
    )
    .with_suggestion("Run the program from a terminal to see why it stopped, then launch it again.")
    .with_disposition(DeliverySemantics::delivered_unverified()))
}

/// The window a launch reports among the windows of its application: one
/// that was not open before the launch, or, for an application the launch
/// started, its first. A window that was already open is not the launch's.
fn choose_window(own: Vec<WindowInfo>, known: &HashSet<String>) -> Option<WindowInfo> {
    let fresh = own.iter().position(|window| !known.contains(&window.id));
    match fresh {
        Some(index) => own.into_iter().nth(index),
        None if known.is_empty() => own.into_iter().next(),
        None => None,
    }
}

/// How long a launch that was not asked to bring the window forward waits for
/// the first window of an application it started.
const WINDOW_GRACE: Duration = Duration::from_millis(2_000);

/// How long such a launch waits for the new window of an application that
/// was already running and forwarded the request.
const NEW_WINDOW_WAIT: Duration = Duration::from_millis(5_000);

struct Launch<'a> {
    bus: &'a SharedBus,
    id: &'a str,
    options: &'a LaunchOptions,
    lease: &'a InteractionLease,
    deadline: Deadline,
}

impl Launch<'_> {
    fn window_of(
        &self,
        owner: &Running,
        waits: bool,
        known: &HashSet<String>,
    ) -> Result<Option<WindowInfo>, AdapterError> {
        let filter = WindowFilter {
            focused_only: false,
            app: Some(owner.name.clone()),
        };
        let limit = self.wait_limit(known);
        loop {
            match list_windows(self.bus, &filter, self.deadline) {
                Ok(windows) => {
                    let own: Vec<WindowInfo> = windows
                        .into_iter()
                        .filter(|window| {
                            window.pid.get() == owner.pid
                                && window.process_instance.as_deref()
                                    == Some(owner.instance.as_str())
                        })
                        .collect();
                    if let Some(window) = choose_window(own, known) {
                        return Ok(Some(window));
                    }
                }
                Err(error)
                    if matches!(
                        error.code,
                        ErrorCode::Timeout
                            | ErrorCode::AppUnresponsive
                            | ErrorCode::AppNotFound
                            | ErrorCode::WindowNotFound
                    ) => {}
                Err(error) => {
                    return Err(error.with_disposition(DeliverySemantics::delivered_unverified()));
                }
            }
            if waits {
                exited_early(owner, liveness(owner.pid, &owner.instance))?;
            }
            if !waits || limit.is_expired() {
                return Ok(None);
            }
            std::thread::sleep(POLL_INTERVAL.min(limit.remaining()));
        }
    }

    fn wait_limit(&self, known: &HashSet<String>) -> Deadline {
        if self.options.activate {
            return self.deadline;
        }
        let wait = if known.is_empty() {
            WINDOW_GRACE
        } else {
            NEW_WINDOW_WAIT
        };
        self.deadline.capped(wait)
    }

    fn known_windows(&self) -> HashSet<String> {
        list_windows(self.bus, &WindowFilter::default(), self.deadline)
            .map(|windows| windows.into_iter().map(|window| window.id).collect())
            .unwrap_or_default()
    }

    fn conclude_with(
        &self,
        owner: &Running,
        window: Option<WindowInfo>,
        created: bool,
    ) -> LaunchResult {
        conclude(owner, window, self.options, created, |window| {
            focus_window(self.bus, window, self.lease)
        })
    }

    fn report(
        &self,
        owner: &Running,
        created: bool,
        known: &HashSet<String>,
    ) -> Result<LaunchResult, AdapterError> {
        let window = self.window_of(owner, true, known)?;
        Ok(self.conclude_with(owner, window, created))
    }

    /// A running application is reported with its window. One that shows no
    /// window, such as a file manager that stays resident after its last
    /// window closed, is started again so that its launcher forwards to it
    /// and a window opens.
    fn attach(
        &self,
        owner: &Running,
        plan: &Plan,
        keys: &[String],
        running: &[Running],
    ) -> Result<LaunchResult, AdapterError> {
        match self.window_of(owner, false, &HashSet::new())? {
            Some(window) => Ok(self.conclude_with(owner, Some(window), false)),
            None => self.start(plan, keys, running),
        }
    }

    fn report_unlisted(&self, spawned: &Spawned) -> LaunchResult {
        let facts = process_facts::read(spawned.pid);
        LaunchResult {
            app: facts
                .as_ref()
                .map_or_else(|| self.id.to_owned(), super::listed_app_name),
            pid: ProcessId::new(spawned.pid),
            process_instance: facts.map(|facts| facts.instance),
            window: None,
            cdp: None,
            renderer: chromium_renderer(spawned.pid),
            suggestion: Some(
                "The process started but is not on the accessibility bus yet. Its pid is the spawned process's and may be a wrapper of the application. Run 'list-apps' to see whether it appears.".to_owned(),
            ),
        }
    }

    fn start(
        &self,
        plan: &Plan,
        keys: &[String],
        running: &[Running],
    ) -> Result<LaunchResult, AdapterError> {
        let before: HashSet<u32> = running.iter().map(|app| app.pid).collect();
        let known = if running.is_empty() {
            HashSet::new()
        } else {
            self.known_windows()
        };
        let mut spawned = spawn(self.id, plan, self.options)?;
        match find_owner(self.bus, &mut spawned, keys, &before, self.deadline)? {
            Found::Owner(owner) => self.report(&owner, owner.pid == spawned.pid, &known),
            Found::NotOnBus => Ok(self.report_unlisted(&spawned)),
        }
    }
}

/// Starts an application by command name, absolute path, desktop id or
/// desktop entry name, or attaches to the running one, and reports the pid of
/// the application that owns the windows with its first window. The attach
/// decision is made before anything is spawned.
pub(crate) fn launch_app(
    bus: &SharedBus,
    id: &str,
    options: &LaunchOptions,
    lease: &InteractionLease,
) -> Result<LaunchResult, AdapterError> {
    validate(options)?;
    let deadline = bounded(lease.deadline(), options.timeout_ms);
    let search = Search::from_env();
    let target = classify(id, &search, &current_index())?;
    let plan = plan(&target, &options.args, &search)?;
    let keys = identity_keys(&plan.keys);
    let running = running_of(&read_registry(bus, deadline).map_err(before_spawn)?);
    let launch = Launch {
        bus,
        id,
        options,
        lease,
        deadline,
    };
    match decide(
        &keys,
        &running,
        options.attach_if_running,
        !options.args.is_empty(),
    ) {
        Decision::Refuse(pids) => Err(before_spawn(
            AdapterError::new(
                ErrorCode::ActionFailed,
                format!("'{id}' is already running (pids {pids:?}) and a new launch was required"),
            )
            .with_suggestion("Allow attaching to use the running instance, or close it first."),
        )),
        Decision::Ambiguous(pids) => Err(AdapterError::ambiguous_process_target(
            format!("'{id}' matches several running applications"),
            &pids_of(&pids),
        )),
        Decision::Attach(owner) => launch.attach(&owner, &plan, &keys, &running),
        Decision::Forward(_) | Decision::Spawn => launch.start(&plan, &keys, &running),
    }
}

#[cfg(test)]
#[path = "launch_tests.rs"]
mod tests;
