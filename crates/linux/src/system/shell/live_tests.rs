use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use agent_desktop_core::{
    Deadline, ErrorCode, InteractionLease, InteractionPolicy, MouseButton, ObservationRequest,
    SnapshotSurface, SystemOps, TreeOptions,
};

use super::container::{Anchor, Landmark, locate_anchor};
use super::identity::find_root;
use super::kinds::ShellKind;
use super::light::LiveNodes;
use super::open::{Opened, close, is_showing, open};
use super::search::{Query, find};
use super::{observe_shell, resolve_surface, supported, window_gate};
use crate::adapter::LinuxAdapter;
use crate::input::shell_click;
use crate::system::SharedBus;
use crate::tree::AtspiElement;

const ATTEMPTS: u32 = 6;
const HEALTHY_ANSWERS: u32 = 3;
const HEALTH_SECONDS: u32 = 30;

struct PrivateShell {
    child: Option<Child>,
}

fn runtime_dir() -> PathBuf {
    let dir = std::env::var("AGENT_DESKTOP_PRIVATE_SESSION")
        .expect("run inside scripts/private-session.sh: this test replaces the desktop shell");
    assert!(
        dir.contains("/rt."),
        "refusing to replace a shell outside a private session"
    );
    PathBuf::from(dir)
}

fn answers(command: &mut Command) -> bool {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn healthy() -> bool {
    let mut in_a_row = 0;
    for _ in 0..HEALTH_SECONDS {
        let window_manager = Command::new("xprop")
            .args(["-root", "_NET_SUPPORTING_WM_CHECK"])
            .output()
            .is_ok_and(|out| String::from_utf8_lossy(&out.stdout).contains("window id"));
        let daemon = answers(Command::new("timeout").args([
            "5",
            "gdbus",
            "call",
            "--session",
            "--dest",
            "org.freedesktop.Notifications",
            "--object-path",
            "/org/freedesktop/Notifications",
            "--method",
            "org.freedesktop.Notifications.GetServerInformation",
        ]));
        if window_manager && daemon {
            in_a_row += 1;
            if in_a_row >= HEALTHY_ANSWERS {
                return true;
            }
        } else {
            in_a_row = 0;
        }
        std::thread::sleep(Duration::from_secs(1));
    }
    false
}

impl PrivateShell {
    fn spawn() -> Child {
        let log = std::fs::File::create(runtime_dir().join("shell.log")).expect("a log file");
        Command::new("gnome-shell")
            .args(["--x11", "--replace"])
            .stdin(Stdio::null())
            .stdout(log.try_clone().expect("a log handle"))
            .stderr(log)
            .process_group(0)
            .spawn()
            .expect("gnome-shell starts")
    }

    fn start() -> Self {
        let mut shell = Self { child: None };
        shell.restart();
        shell
    }

    fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
            std::thread::sleep(Duration::from_secs(2));
        }
    }

    fn restart(&mut self) {
        for _ in 0..ATTEMPTS {
            self.stop();
            self.child = Some(Self::spawn());
            if healthy() {
                std::thread::sleep(Duration::from_secs(2));
                return;
            }
        }
        panic!("no healthy private gnome-shell after {ATTEMPTS} starts");
    }
}

impl Drop for PrivateShell {
    fn drop(&mut self) {
        self.stop();
    }
}

fn headed_lease() -> InteractionLease {
    let deadline = Deadline::after(120_000).expect("a deadline builds");
    SystemOps::acquire_interaction_lease(&LinuxAdapter::new(), deadline)
        .expect("the private session grants the lease")
}

pub(super) fn deadline() -> Deadline {
    Deadline::after(30_000).expect("a deadline builds")
}

pub(super) fn named(
    bus: &SharedBus,
    name: &'static str,
    role: u32,
) -> Option<crate::system::shell::light::Light> {
    let root = find_root(bus, deadline()).expect("the shell root");
    let nodes = LiveNodes::new(
        bus.get(deadline()).expect("bus").inner().clone(),
        deadline(),
    );
    let start = AtspiElement::new(root.bus_name.as_str(), root.path.as_str(), root.pid);
    let candidate = move |light: &crate::system::shell::light::Light| light.is(role, name);
    let found = find(
        &nodes,
        &start,
        deadline(),
        &Query {
            prune: &|_| false,
            candidate: &candidate,
            accept: &|_, light| light.placed().is_some() || name == "Overview",
        },
    )
    .expect("the search runs");
    found.and_then(|path| path.last().cloned())
}

fn close_the_overview(bus: &SharedBus, lease: &InteractionLease) {
    let showing = |bus: &SharedBus| named(bus, "Overview", 39).is_some_and(|node| node.showing);
    if !showing(bus) {
        return;
    }
    let activities = named(bus, "Activities", 62).expect("the Activities button is placed");
    shell_click::click(
        bus,
        &activities.element,
        MouseButton::Left,
        InteractionPolicy::headed(),
        lease,
    )
    .expect("the click on Activities lands");
    let until = Instant::now() + Duration::from_secs(8);
    while showing(bus) && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(200));
    }
    assert!(!showing(bus), "the overview closes");
}

fn observed_text(bus: &SharedBus, surface: SnapshotSurface) -> String {
    let window = resolve_surface(bus, surface, deadline()).expect("the surface resolves");
    let request = ObservationRequest::snapshot(&TreeOptions::default(), deadline());
    let tree = observe_shell(bus, &window, &request).expect("the surface observes");
    let (root, complete, _) = tree
        .into_accessibility_tree_partial()
        .expect("one root projects");
    assert!(complete, "the observation is complete");
    serde_json::to_string(&root).expect("the tree serialises")
}

#[test]
#[ignore = "replaces the shell of a private session; run inside scripts/private-session.sh"]
fn shell_surfaces_observe_open_close_and_follow_a_restart() {
    let mut shell = PrivateShell::start();
    let bus = SharedBus::default();
    let lease = headed_lease();
    close_the_overview(&bus, &lease);

    let surfaces = supported(&bus);
    for wanted in [
        SnapshotSurface::SystemTray,
        SnapshotSurface::ActionCenter,
        SnapshotSurface::QuickSettings,
    ] {
        assert!(
            surfaces.contains(&wanted),
            "{wanted:?} is advertised: {surfaces:?}"
        );
    }

    assert!(observed_text(&bus, SnapshotSurface::SystemTray).contains("System"));
    assert!(observed_text(&bus, SnapshotSurface::ActionCenter).contains("Clear all notifications"));
    assert!(observed_text(&bus, SnapshotSurface::QuickSettings).contains("Power Off Menu"));

    for kind in [ShellKind::SystemMenu, ShellKind::MessageList] {
        assert!(
            !is_showing(&bus, kind, deadline()).expect("a read"),
            "{kind:?} starts closed"
        );
        let headless = open(&bus, kind, InteractionPolicy::headless(), &lease)
            .expect_err("headless cannot click");
        assert_eq!(headless.code, ErrorCode::PolicyDenied);
        assert!(
            !is_showing(&bus, kind, deadline()).expect("a read"),
            "{kind:?} untouched"
        );

        let (window, how) = open(&bus, kind, InteractionPolicy::headed(), &lease)
            .expect("the headed click opens it");
        assert_eq!(how, Opened::Opened);
        assert!(
            is_showing(&bus, kind, deadline()).expect("a read"),
            "{kind:?} stays open"
        );
        window_gate::verify(&bus, &window.id, deadline()).expect("the id is live while open");

        let (_, again) = open(&bus, kind, InteractionPolicy::headless(), &lease)
            .expect("an open surface is adopted without a click");
        assert_eq!(again, Opened::Adopted);

        close(&bus, kind, &lease).expect("the click closes it");
        assert!(
            !is_showing(&bus, kind, deadline()).expect("a read"),
            "{kind:?} closed again"
        );
    }

    let before = resolve_surface(&bus, SnapshotSurface::SystemTray, deadline()).expect("resolves");
    let list = locate_anchor(
        &bus,
        &find_root(&bus, deadline()).expect("root"),
        Anchor::Landmark(Landmark::MessageList),
        deadline(),
    )
    .expect("the list is located");
    assert!(!list.anchor.showing);

    shell.restart();
    let stale = window_gate::verify(&bus, &before.id, deadline())
        .expect_err("an id from before the restart is refused");
    assert_eq!(stale.code, ErrorCode::StaleRef);
    let after = resolve_surface(&bus, SnapshotSurface::SystemTray, deadline())
        .expect("the new shell resolves");
    assert_ne!(after.id, before.id);
    close_the_overview(&bus, &lease);
    let (_, how) = open(
        &bus,
        ShellKind::SystemMenu,
        InteractionPolicy::headed(),
        &lease,
    )
    .expect("the new shell opens its menu");
    assert_eq!(how, Opened::Opened);
    close(&bus, ShellKind::SystemMenu, &lease).expect("and closes it");
}
