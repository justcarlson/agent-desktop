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
        nodes.push(node(1, Role::Notification, "", &format!("/e{n}"), LAID_OUT));
        nodes.push(node(2, Role::Other, "", &format!("/e{n}/header"), LAID_OUT));
        nodes.push(node(
            3,
            Role::Label,
            entry.app,
            &format!("/e{n}/app"),
            LAID_OUT,
        ));
        nodes.push(node(
            3,
            Role::PushButton,
            "",
            &format!("/e{n}/expand"),
            LAID_OUT,
        ));
        nodes.push(node(
            3,
            Role::PushButton,
            "",
            &format!("/e{n}/close"),
            LAID_OUT,
        ));
        if let Some(time) = entry.time {
            nodes.push(node(3, Role::Label, time, &format!("/e{n}/time"), LAID_OUT));
        }
        nodes.push(node(
            2,
            Role::Other,
            "",
            &format!("/e{n}/content"),
            LAID_OUT,
        ));
        nodes.push(node(
            3,
            Role::Label,
            entry.title,
            &format!("/e{n}/title"),
            LAID_OUT,
        ));
        if let Some(body) = entry.body {
            nodes.push(node(3, Role::Label, body, &format!("/e{n}/body"), LAID_OUT));
        }
        for action in &entry.actions {
            let extents = if entry.expanded { LAID_OUT } else { HIDDEN };
            let path = format!("/e{n}/act/{action}");
            nodes.push(node(3, Role::PushButton, action, &path, extents));
            nodes.push(node(
                4,
                Role::Label,
                action,
                &format!("{path}/label"),
                extents,
            ));
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

pub(crate) struct FakeWorld {
    pub(crate) log: Log,
    stages: Vec<Reading>,
    clicks: Cell<usize>,
    reads: Cell<usize>,
    by_reads: bool,
    pub(crate) opened: Opened,
    pub(crate) open_fails: bool,
    pub(crate) showing: Cell<bool>,
    pub(crate) close_sticks: bool,
    pub(crate) click_fails: bool,
}

impl FakeWorld {
    pub(crate) fn new(log: &Log, stages: Vec<Reading>) -> Self {
        Self {
            log: Rc::clone(log),
            stages,
            clicks: Cell::new(0),
            reads: Cell::new(0),
            by_reads: false,
            opened: Opened::Opened,
            open_fails: false,
            showing: Cell::new(false),
            close_sticks: false,
            click_fails: false,
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
    fn locate(&self, landmark: Landmark, _: Deadline) -> Result<AtspiElement, AdapterError> {
        self.note(format!("locate:{landmark:?}"));
        Ok(element("/root"))
    }

    fn read(&self, _: &AtspiElement, _: Deadline) -> Result<Reading, AdapterError> {
        self.note("read");
        let position = if self.by_reads {
            self.reads.get()
        } else {
            self.clicks.get()
        };
        self.reads.set(self.reads.get() + 1);
        let stage = position.min(self.stages.len() - 1);
        Ok(self.stages[stage].clone())
    }

    fn open(
        &self,
        _: InteractionPolicy,
        _: &InteractionLease,
    ) -> Result<(WindowInfo, Opened), AdapterError> {
        self.note("open");
        if self.open_fails {
            self.showing.set(true);
            return Err(AdapterError::timeout("the list never showed"));
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
        if !self.close_sticks {
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
        if self.click_fails {
            return Err(AdapterError::new(
                ErrorCode::ActionFailed,
                "the click failed",
            ));
        }
        self.clicks.set(self.clicks.get() + 1);
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
