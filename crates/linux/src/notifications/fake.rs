use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use agent_desktop_core::{
    AdapterError, Deadline, ErrorCode, InteractionLease, InteractionPolicy, MouseButton, Point,
    ProcessId, WindowInfo, WindowState,
};

use super::signals::{Signal, SignalSource, Subscription};
use super::world::{Node, Reading, Role, ShellWorld};
use crate::system::shell::container::Landmark;
use crate::system::shell::open::Opened;
use crate::tree::{AtspiElement, Extents};

pub(crate) type Log = Rc<RefCell<Vec<String>>>;

pub(crate) fn log() -> Log {
    Rc::new(RefCell::new(Vec::new()))
}

pub(crate) fn lease(millis: u64) -> InteractionLease {
    InteractionLease::guarded(Deadline::after(millis).expect("a deadline"), ()).expect("a lease")
}

pub(crate) fn element(path: &str) -> AtspiElement {
    AtspiElement::new(":1.1", path, 1)
}

pub(crate) struct Spec {
    pub(crate) app: &'static str,
    pub(crate) title: &'static str,
    pub(crate) body: Option<&'static str>,
    pub(crate) time: Option<&'static str>,
    pub(crate) actions: Vec<&'static str>,
    pub(crate) expanded: bool,
}

pub(crate) fn spec(app: &'static str, title: &'static str) -> Spec {
    Spec {
        app,
        title,
        body: None,
        time: None,
        actions: Vec::new(),
        expanded: false,
    }
}

impl Spec {
    pub(crate) fn body(mut self, body: &'static str) -> Self {
        self.body = Some(body);
        self
    }

    pub(crate) fn time(mut self, time: &'static str) -> Self {
        self.time = Some(time);
        self
    }

    pub(crate) fn actions(mut self, actions: &[&'static str]) -> Self {
        self.actions = actions.to_vec();
        self
    }

    pub(crate) fn expanded(mut self) -> Self {
        self.expanded = true;
        self
    }
}

const LAID_OUT: Extents = Extents {
    x: 100,
    y: 100,
    width: 50,
    height: 20,
};
const HIDDEN: Extents = Extents {
    x: i32::MIN,
    y: i32::MIN,
    width: -1,
    height: -1,
};

fn node(depth: usize, role: Role, name: &str, path: &str, extents: Extents) -> Node {
    Node {
        depth,
        role,
        name: name.to_owned(),
        extents: Some(extents),
        element: element(path),
    }
}

pub(crate) fn reading(specs: &[Spec], clear_all: bool) -> Reading {
    let mut nodes = vec![node(0, Role::Other, "", "/root", LAID_OUT)];
    for (position, entry) in specs.iter().enumerate() {
        let n = position + 1;
        let mut push = |depth, role, name, suffix: &str, extents| {
            nodes.push(node(depth, role, name, &format!("/e{n}{suffix}"), extents));
        };
        push(1, Role::Notification, "", "", LAID_OUT);
        push(2, Role::Other, "", "/header", LAID_OUT);
        push(3, Role::Label, entry.app, "/app", LAID_OUT);
        push(3, Role::PushButton, "", "/expand", LAID_OUT);
        push(3, Role::PushButton, "", "/close", LAID_OUT);
        if let Some(time) = entry.time {
            push(3, Role::Label, time, "/time", LAID_OUT);
        }
        push(2, Role::Other, "", "/content", LAID_OUT);
        push(3, Role::Label, entry.title, "/title", LAID_OUT);
        if let Some(body) = entry.body {
            push(3, Role::Label, body, "/body", LAID_OUT);
        }
        for action in &entry.actions {
            let extents = if entry.expanded { LAID_OUT } else { HIDDEN };
            let suffix = format!("/act/{action}");
            push(3, Role::PushButton, action, &suffix, extents);
            push(4, Role::Label, action, &format!("{suffix}/label"), extents);
        }
    }
    if specs.is_empty() {
        nodes.push(node(1, Role::Label, "No Notifications", "/empty", LAID_OUT));
    }
    if clear_all {
        nodes.push(node(
            1,
            Role::PushButton,
            "Clear all notifications",
            "/clear",
            LAID_OUT,
        ));
    }
    Reading {
        nodes,
        complete: true,
    }
}

#[derive(Default)]
pub(crate) struct Faults {
    pub(crate) daemon: Option<DaemonFailure>,
    pub(crate) open: bool,
    pub(crate) window_info: bool,
    pub(crate) close_sticks: bool,
    pub(crate) click: bool,
    pub(crate) read_after_click: bool,
}

pub(crate) enum DaemonFailure {
    Absent,
    Unsupported,
}

#[derive(Default)]
struct Counts {
    clicks: Cell<usize>,
    reads: Cell<usize>,
}

pub(crate) struct FakeWorld {
    pub(crate) log: Log,
    stages: Vec<Reading>,
    counts: Counts,
    by_reads: bool,
    pub(crate) opened: Opened,
    pub(crate) showing: Cell<bool>,
    pub(crate) faults: Faults,
}

impl FakeWorld {
    pub(crate) fn new(log: &Log, stages: Vec<Reading>) -> Self {
        Self {
            log: Rc::clone(log),
            stages,
            counts: Counts::default(),
            by_reads: false,
            opened: Opened::Opened,
            showing: Cell::new(false),
            faults: Faults::default(),
        }
    }

    pub(crate) fn staged_by_reads(mut self) -> Self {
        self.by_reads = true;
        self
    }

    pub(crate) fn adopted(mut self) -> Self {
        self.opened = Opened::Adopted;
        self.showing.set(true);
        self
    }

    pub(crate) fn clicks(&self) -> Vec<String> {
        self.log
            .borrow()
            .iter()
            .filter_map(|line| line.strip_prefix("click:").map(str::to_owned))
            .collect()
    }

    fn note(&self, line: impl Into<String>) {
        self.log.borrow_mut().push(line.into());
    }
}

impl ShellWorld for FakeWorld {
    fn require_daemon(&self, _: Deadline) -> Result<(), AdapterError> {
        use crate::system::shell::detect::{Fact, Need, ShellEnvironment, require};

        let (daemon, server) = match &self.faults.daemon {
            Some(DaemonFailure::Absent) => (
                Fact::Absent("no service owns org.freedesktop.Notifications".to_owned()),
                None,
            ),
            Some(DaemonFailure::Unsupported) => (
                Fact::Absent("unsupported notification service".to_owned()),
                Some("other".to_owned()),
            ),
            None => (Fact::Present, Some("gnome-shell".to_owned())),
        };
        let environment = ShellEnvironment {
            daemon,
            tree: Fact::Present,
            watcher: Fact::Present,
            server,
        };
        require(&environment, Need::Daemon)
    }

    fn locate(&self, landmark: Landmark, _: Deadline) -> Result<AtspiElement, AdapterError> {
        self.note(format!("locate:{landmark:?}"));
        Ok(element("/root"))
    }

    fn read(&self, _: &AtspiElement, _: Deadline) -> Result<Reading, AdapterError> {
        self.note("read");
        if self.faults.read_after_click && self.counts.clicks.get() > 0 {
            return Err(AdapterError::new(
                ErrorCode::Internal,
                "the shell tree could not be read",
            ));
        }
        let position = if self.by_reads {
            self.counts.reads.get()
        } else {
            self.counts.clicks.get()
        };
        self.counts.reads.set(self.counts.reads.get() + 1);
        let stage = position.min(self.stages.len() - 1);
        Ok(self.stages[stage].clone())
    }

    fn open(
        &self,
        _: InteractionPolicy,
        _: &InteractionLease,
    ) -> Result<(WindowInfo, Opened), AdapterError> {
        self.note("open");
        if self.faults.open || self.faults.window_info {
            let opened_by_call = self.opened == Opened::Opened;
            self.showing.set(true);
            let message = if self.faults.window_info {
                "window info could not be read"
            } else {
                "the list never showed"
            };
            return Err(AdapterError::timeout(message)
                .with_details(serde_json::json!({"surface_opened_by_call": opened_by_call})));
        }
        self.showing.set(true);
        let window = WindowInfo {
            id: "w".to_owned(),
            title: String::new(),
            app: String::new(),
            pid: ProcessId::new(1),
            process_instance: None,
            bounds: None,
            state: WindowState::default(),
        };
        Ok((window, self.opened))
    }

    fn close(&self, lease: &InteractionLease) -> Result<(), AdapterError> {
        self.note("close");
        if lease.deadline().is_expired() {
            return Err(AdapterError::timeout("the close ran on a spent deadline"));
        }
        if !self.faults.close_sticks {
            self.showing.set(false);
        }
        Ok(())
    }

    fn is_showing(&self, _: Deadline) -> Result<bool, AdapterError> {
        self.note("showing");
        Ok(self.showing.get())
    }

    fn click(
        &self,
        node: &AtspiElement,
        _: MouseButton,
        _: InteractionPolicy,
        _: &InteractionLease,
    ) -> Result<Point, AdapterError> {
        self.note(format!("click:{}", node.path()));
        if self.faults.click {
            return Err(AdapterError::new(
                ErrorCode::ActionFailed,
                "the click failed",
            ));
        }
        self.counts.clicks.set(self.counts.clicks.get() + 1);
        Ok(Point { x: 0.0, y: 0.0 })
    }
}

pub(crate) struct FakeSignals {
    pub(crate) log: Log,
    pub(crate) signals: Vec<Signal>,
    pub(crate) fails: bool,
}

struct Held(Vec<Signal>);

impl Subscription for Held {
    fn collect(self: Box<Self>, _: Duration) -> Vec<Signal> {
        self.0
    }
}

impl SignalSource for FakeSignals {
    fn subscribe(&self, _: Deadline) -> Result<Box<dyn Subscription>, AdapterError> {
        self.log.borrow_mut().push("subscribe".to_owned());
        if self.fails {
            return Err(AdapterError::new(ErrorCode::Internal, "no session bus"));
        }
        Ok(Box::new(Held(self.signals.clone())))
    }
}
