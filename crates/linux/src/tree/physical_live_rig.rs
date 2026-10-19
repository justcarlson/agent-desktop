use std::time::{Duration, Instant};

use agent_desktop_core::{
    Action, ActionOps, ActionRequest, ActionResult, AdapterError, Deadline, InteractionLease,
    NativeHandle, ObservationOps, SystemOps,
};

use super::AtspiElement;
use super::live_launch::{Launched, nonce};
use super::live_tree::pid_frame;
use super::resolve_live_tests::snapshot_entries;
use super::walker_live_tests::settle;
use crate::LinuxAdapter;
use crate::input::x11::live_tests::private_display;
use crate::system::SharedBus;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Kit {
    Gtk,
    Qt,
}

pub(super) struct Scene {
    pub(super) adapter: LinuxAdapter,
    pub(super) bus: SharedBus,
    pub(super) launched: Launched,
    pub(super) frame: AtspiElement,
    pub(super) log: String,
    pub(super) pid: u32,
}

pub(super) struct Tgt {
    pub(super) handle: NativeHandle,
}

pub(super) fn deadline(ms: u64) -> Deadline {
    Deadline::after(ms).expect("the deadline builds")
}

fn qt_launch(launched: &mut Launched, script: &str, log: &str) -> u32 {
    let home = std::env::var("HOME").expect("HOME");
    let python = format!("{home}/.local/ad-tools/qtvenv/bin/python");
    let libraries = format!("{home}/.local/ad-tools/root/usr/lib/x86_64-linux-gnu");
    launched.spawn_with_env(
        &python,
        &[script],
        &[
            ("AD_LIVE_LOG", log.to_owned()),
            ("PYTHONUTF8", "1".to_owned()),
            ("QT_ACCESSIBILITY", "1".to_owned()),
            ("QT_LINUX_ACCESSIBILITY_ALWAYS_ON", "1".to_owned()),
            ("QT_QPA_PLATFORM", "xcb".to_owned()),
            ("LD_LIBRARY_PATH", libraries),
        ],
    )
}

pub(super) fn write_script(source: &str) -> (String, String) {
    let dir = std::env::var("AGENT_DESKTOP_PRIVATE_SESSION").expect("session");
    let tag = nonce("phys");
    let script = format!("{dir}/{tag}.py");
    std::fs::write(&script, source).expect("script");
    (script, format!("{dir}/{tag}.log"))
}

impl Scene {
    pub(super) fn open(kit: Kit, source: &str, min_nodes: usize) -> Self {
        private_display();
        let (script, log) = write_script(source);
        let mut launched = Launched::new();
        let pid = match kit {
            Kit::Gtk => launched.spawn_with_env(
                "python3",
                &[&script],
                &[("AD_LIVE_LOG", log.clone()), ("PYTHONUTF8", "1".to_owned())],
            ),
            Kit::Qt => qt_launch(&mut launched, &script, &log),
        };
        Self::attach(launched, pid, log, min_nodes)
    }

    pub(super) fn attach(mut launched: Launched, pid: u32, log: String, min_nodes: usize) -> Self {
        let adapter = LinuxAdapter::new();
        let bus = SharedBus::default();
        let connection = bus
            .get(Deadline::standard().expect("deadline"))
            .expect("the accessibility bus is reachable");
        let settled =
            settle(&bus, min_nodes, || pid_frame(&connection, pid)).unwrap_or_else(|error| {
                launched.stop();
                panic!("{error}")
            });
        Self {
            adapter,
            bus,
            launched,
            frame: settled.frame,
            log,
            pid,
        }
    }

    pub(super) fn find(&self, roles: &[&str], name: Option<&str>) -> Tgt {
        let entries = snapshot_entries(&self.adapter, &self.frame, true).expect("a snapshot");
        let summary: Vec<String> = entries
            .iter()
            .map(|(_, entry)| format!("{}:{:?}", entry.identity.role, entry.identity.name))
            .collect();
        let entry = entries
            .into_iter()
            .map(|(_, entry)| entry)
            .find(|entry| {
                roles.contains(&entry.identity.role.as_str())
                    && name.is_none_or(|wanted| entry.identity.name.as_deref() == Some(wanted))
            })
            .unwrap_or_else(|| panic!("no {roles:?} {name:?} among {summary:?}"));
        let handle = self
            .adapter
            .resolve_element_strict(&entry, deadline(10_000))
            .expect("the entry resolves");
        Tgt { handle }
    }

    pub(super) fn lease(&self) -> InteractionLease {
        self.adapter
            .acquire_interaction_lease(deadline(30_000))
            .expect("the lease")
    }

    pub(super) fn act(
        &self,
        handle: &NativeHandle,
        action: Action,
    ) -> Result<ActionResult, AdapterError> {
        let lease = self.lease();
        self.adapter
            .execute_action(handle, ActionRequest::headed(action), &lease)
    }

    pub(super) fn adapter_bounds(&self, target: &Tgt) -> Option<agent_desktop_core::Rect> {
        self.adapter
            .get_element_bounds(&target.handle, deadline(5_000))
            .ok()
            .flatten()
    }

    pub(super) fn bounds_y(&self, target: &Tgt) -> f64 {
        self.adapter
            .get_element_bounds(&target.handle, deadline(5_000))
            .expect("bounds read")
            .expect("bounds present")
            .y
    }

    pub(super) fn settled_y(&self, target: &Tgt) -> f64 {
        let mut last = self.bounds_y(target);
        for _ in 0..30 {
            std::thread::sleep(Duration::from_millis(150));
            let now = self.bounds_y(target);
            if (now - last).abs() < 0.5 {
                return now;
            }
            last = now;
        }
        last
    }

    pub(super) fn log_text(&self) -> String {
        std::fs::read_to_string(&self.log).unwrap_or_default()
    }

    pub(super) fn wait_log(&self, done: impl Fn(&str) -> bool) -> String {
        let started = Instant::now();
        loop {
            let text = self.log_text();
            if done(&text) || started.elapsed() > Duration::from_secs(8) {
                return text;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    pub(super) fn value_of(&self, target: &Tgt) -> Option<String> {
        self.adapter
            .get_live_value(&target.handle, deadline(5_000))
            .ok()
            .flatten()
    }
}

impl Drop for Scene {
    fn drop(&mut self) {
        self.launched.stop();
    }
}
