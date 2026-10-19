use std::time::{Duration, Instant};

use agent_desktop_core::{
    Action, ActionRequest, ActionResult, AdapterError, Deadline, NativeHandle, ObservationOps,
    SystemOps, WindowFilter,
};
use zbus::zvariant::OwnedValue;

use crate::LinuxAdapter;
use crate::system::{PROPERTIES_INTERFACE, window_id, within};
use crate::tree::live_launch::{Launched, nonce};
use crate::tree::{AtspiElement, list_children, read_node};

const GTK3_SCRIPT: &str = r#"
import sys
import gi
gi.require_version("Gtk", "3.0")
from gi.repository import Gtk

window = Gtk.Window(title="adv-" + sys.argv[1])
window.connect("destroy", Gtk.main_quit)
box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
field = Gtk.Entry()
field.get_accessible().set_name("field")
box.add(field)
secret = Gtk.Entry()
secret.set_visibility(False)
secret.get_accessible().set_name("secret")
box.add(secret)
spin = Gtk.SpinButton.new_with_range(0, 100, 1)
spin.get_accessible().set_name("spin")
box.add(spin)
scale = Gtk.Scale.new_with_range(Gtk.Orientation.HORIZONTAL, 0, 100, 1)
scale.get_accessible().set_name("scale")
box.add(scale)
pane = Gtk.ScrolledWindow()
pane.set_size_request(200, 100)
pane.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.ALWAYS)
pane.add(Gtk.Label(label="\n".join(str(n) for n in range(200))))
box.add(pane)
window.add(box)
window.show_all()
Gtk.main()
"#;

const GTK4_SCRIPT: &str = r#"
import sys
import gi
gi.require_version("Gtk", "4.0")
from gi.repository import Gtk

def label(widget, text):
    widget.update_property([Gtk.AccessibleProperty.LABEL], [text])

def activate(app):
    window = Gtk.ApplicationWindow(application=app, title="adv-" + sys.argv[1])
    box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
    field = Gtk.Entry()
    label(field, "field")
    box.append(field)
    secret = Gtk.PasswordEntry()
    label(secret, "secret")
    box.append(secret)
    spin = Gtk.SpinButton.new_with_range(0, 100, 1)
    label(spin, "spin")
    box.append(spin)
    scale = Gtk.Scale.new_with_range(Gtk.Orientation.HORIZONTAL, 0, 100, 1)
    label(scale, "scale")
    box.append(scale)
    bar = Gtk.Scrollbar(orientation=Gtk.Orientation.VERTICAL, adjustment=Gtk.Adjustment(value=0, lower=0, upper=1000, step_increment=10, page_increment=100, page_size=100))
    box.append(bar)
    window.set_child(box)
    window.present()

app = Gtk.Application(application_id="org.agentdesktop.ValueFixture4")
app.connect("activate", activate)
app.run([])
"#;

const QT_SCRIPT: &str = r#"
import sys
from PyQt6.QtWidgets import (QApplication, QWidget, QVBoxLayout, QLineEdit, QSpinBox, QSlider, QScrollBar)
from PyQt6.QtCore import Qt
app = QApplication(sys.argv)
w = QWidget()
w.setWindowTitle("adv-" + sys.argv[1])
lay = QVBoxLayout(w)
field = QLineEdit()
field.setAccessibleName("field")
lay.addWidget(field)
secret = QLineEdit()
secret.setEchoMode(QLineEdit.EchoMode.Password)
secret.setAccessibleName("secret")
lay.addWidget(secret)
spin = QSpinBox()
spin.setRange(0, 100)
spin.setAccessibleName("spin")
lay.addWidget(spin)
scale = QSlider(Qt.Orientation.Horizontal)
scale.setRange(0, 100)
scale.setAccessibleName("scale")
lay.addWidget(scale)
bar = QScrollBar(Qt.Orientation.Vertical)
bar.setRange(0, 1000)
lay.addWidget(bar)
w.show()
sys.exit(app.exec())
"#;

#[derive(Clone, Copy, Debug)]
pub(super) enum Toolkit {
    Gtk3,
    Gtk4,
    Qt,
}

pub(super) const TOOLKITS: [Toolkit; 3] = [Toolkit::Gtk3, Toolkit::Gtk4, Toolkit::Qt];

pub(super) struct Bench {
    adapter: LinuxAdapter,
    launched: Launched,
    frame: AtspiElement,
}

fn deadline(ms: u64) -> Deadline {
    Deadline::after(ms).expect("the deadline builds")
}

type Environment = Vec<(&'static str, String)>;

fn qt_environment() -> Result<(String, Environment), String> {
    let home = std::env::var("HOME").map_err(|error| error.to_string())?;
    let python = format!("{home}/.local/ad-tools/qtvenv/bin/python");
    if !std::path::Path::new(&python).exists() {
        return Err(format!("the Qt test environment is missing at {python}"));
    }
    let libraries = format!("{home}/.local/ad-tools/root/usr/lib/x86_64-linux-gnu");
    let library_path = match std::env::var("LD_LIBRARY_PATH") {
        Ok(existing) if !existing.is_empty() => format!("{libraries}:{existing}"),
        _ => libraries,
    };
    let environment = vec![
        ("QT_ACCESSIBILITY", "1".to_owned()),
        ("QT_LINUX_ACCESSIBILITY_ALWAYS_ON", "1".to_owned()),
        ("QT_QPA_PLATFORM", "xcb".to_owned()),
        ("LD_LIBRARY_PATH", library_path),
    ];
    Ok((python, environment))
}

impl Bench {
    pub(super) fn open(toolkit: Toolkit) -> Result<Self, String> {
        let adapter = LinuxAdapter::new();
        let mut launched = Launched::new();
        let tag = nonce("value");
        let dir = std::env::temp_dir().join(&tag);
        std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
        launched.scratch.push(dir.clone());
        let script = dir.join("app.py");
        let (program, source, environment) = match toolkit {
            Toolkit::Gtk3 => ("python3".to_owned(), GTK3_SCRIPT, Vec::new()),
            Toolkit::Gtk4 => ("python3".to_owned(), GTK4_SCRIPT, Vec::new()),
            Toolkit::Qt => {
                let (python, environment) = qt_environment()?;
                (python, QT_SCRIPT, environment)
            }
        };
        std::fs::write(&script, source).map_err(|error| error.to_string())?;
        let script_arg = script.to_string_lossy().into_owned();
        let pid = launched.spawn_with_env(&program, &[&script_arg, &tag], &environment);
        let frame = Self::wait_for_frame(&adapter, &tag, pid)?;
        Ok(Self {
            adapter,
            launched,
            frame,
        })
    }

    fn wait_for_frame(adapter: &LinuxAdapter, tag: &str, pid: u32) -> Result<AtspiElement, String> {
        let started = Instant::now();
        while started.elapsed() < Duration::from_secs(20) {
            let windows = adapter
                .list_windows(&WindowFilter::default(), deadline(5_000))
                .unwrap_or_default();
            if let Some(window) = windows.iter().find(|window| window.title.contains(tag)) {
                let (bus, path) = window_id::decode(&window.id).map_err(|e| e.message)?;
                return Ok(AtspiElement::new(bus, path, window.pid.get()));
            }
            if let Ok(connection) = adapter.bus().get(deadline(5_000))
                && let Some(frame) = super::live_click_tests::frame_of(&connection, pid)
            {
                return Ok(frame);
            }
            std::thread::sleep(Duration::from_millis(250));
        }
        Err("the fixture window never appeared".to_owned())
    }

    fn walk_all(&self, matches: impl Fn(&crate::tree::RawNode) -> bool) -> Vec<AtspiElement> {
        let budget = deadline(10_000);
        let Ok(connection) = self.adapter.bus().get(budget) else {
            return Vec::new();
        };
        let mut root = self.frame.clone();
        root.set_ancestry(crate::tree::ancestry::of_root(&root));
        let mut queue = vec![root];
        let mut found = Vec::new();
        let mut index = 0;
        while index < queue.len() && index < 400 {
            let element = queue[index].clone();
            index += 1;
            let Ok(node) = within(budget, read_node(connection.inner(), &element)) else {
                break;
            };
            if matches(&node) && index > 1 {
                found.push(element.clone());
            }
            let Ok(children) = within(budget, list_children(connection.inner(), &element)) else {
                break;
            };
            let role = node.traits.role.present().copied();
            queue.extend(children.into_iter().map(|(bus, path)| {
                let mut child = AtspiElement::new(bus, path, self.frame.pid());
                child.set_ancestry(crate::tree::ancestry::child(&element, role));
                child
            }));
        }
        found
    }

    fn walk(&self, matches: impl Fn(&crate::tree::RawNode) -> bool) -> Option<AtspiElement> {
        self.walk_all(matches).into_iter().next()
    }

    pub(super) fn named(&self, name: &str) -> NativeHandle {
        let started = Instant::now();
        loop {
            let found = self.walk(|node| node.properties.name.present().is_some_and(|n| n == name));
            if let Some(element) = found {
                return element.into_native_handle();
            }
            assert!(
                started.elapsed() < Duration::from_secs(10),
                "no node named {name}"
            );
            std::thread::sleep(Duration::from_millis(250));
        }
    }

    pub(super) fn all_with_role(&self, role: u32) -> Vec<NativeHandle> {
        self.walk_all(|node| node.traits.role.present() == Some(&role))
            .into_iter()
            .map(AtspiElement::into_native_handle)
            .collect()
    }

    pub(super) fn with_role(&self, role: u32) -> Option<NativeHandle> {
        self.walk(|node| node.traits.role.present() == Some(&role))
            .map(AtspiElement::into_native_handle)
    }

    pub(super) fn act(
        &self,
        handle: &NativeHandle,
        action: Action,
    ) -> Result<ActionResult, AdapterError> {
        let lease = SystemOps::acquire_interaction_lease(&self.adapter, deadline(30_000))?;
        crate::actions::execute_action(
            self.adapter.bus(),
            handle,
            ActionRequest::headless(action),
            &lease,
        )
    }

    fn property(&self, handle: &NativeHandle, interface: &str, name: &str) -> OwnedValue {
        let element = crate::tree::atspi_element(handle).expect("an element");
        let budget = deadline(5_000);
        let connection = self.adapter.bus().get(budget).expect("the bus");
        within(budget, async {
            let reply = connection
                .inner()
                .call_method(
                    Some(element.bus_name()),
                    element.path(),
                    Some(PROPERTIES_INTERFACE),
                    "Get",
                    &(interface, name),
                )
                .await
                .map_err(|error| AdapterError::internal(error.to_string()))?;
            reply
                .body()
                .deserialize::<OwnedValue>()
                .map_err(|error| AdapterError::internal(error.to_string()))
        })
        .expect("the property reads")
    }

    pub(super) fn text_of(&self, handle: &NativeHandle) -> String {
        let element = crate::tree::atspi_element(handle).expect("an element");
        let count = self
            .property(handle, "org.a11y.atspi.Text", "CharacterCount")
            .downcast_ref::<i32>()
            .expect("a count");
        let budget = deadline(5_000);
        let connection = self.adapter.bus().get(budget).expect("the bus");
        within(budget, async {
            let reply = connection
                .inner()
                .call_method(
                    Some(element.bus_name()),
                    element.path(),
                    Some("org.a11y.atspi.Text"),
                    "GetText",
                    &(0i32, count),
                )
                .await
                .map_err(|error| AdapterError::internal(error.to_string()))?;
            reply
                .body()
                .deserialize::<String>()
                .map_err(|error| AdapterError::internal(error.to_string()))
        })
        .expect("the text reads")
    }

    pub(super) fn maximum_of(&self, handle: &NativeHandle) -> f64 {
        self.property(handle, "org.a11y.atspi.Value", "MaximumValue")
            .downcast_ref::<f64>()
            .expect("a number")
    }

    pub(super) fn number_of(&self, handle: &NativeHandle) -> f64 {
        self.property(handle, "org.a11y.atspi.Value", "CurrentValue")
            .downcast_ref::<f64>()
            .expect("a number")
    }
}

impl Drop for Bench {
    fn drop(&mut self) {
        self.launched.stop();
    }
}

pub(super) fn each_toolkit(check: impl Fn(Toolkit, &Bench)) {
    for toolkit in TOOLKITS {
        match Bench::open(toolkit) {
            Ok(bench) => check(toolkit, &bench),
            Err(reason) => panic!("{toolkit:?} fixture unavailable: {reason}"),
        }
    }
}

fn verified(result: &ActionResult) -> Vec<Option<bool>> {
    result.steps.iter().map(|step| step.verified).collect()
}

#[test]
#[ignore = "needs lane A's write engine; run through scripts/private-session.sh --wm"]
fn set_value_and_clear_land_on_a_text_field_in_every_toolkit() {
    each_toolkit(|toolkit, bench| {
        let field = bench.named("field");
        let result = bench
            .act(&field, Action::SetValue("hello".to_owned()))
            .unwrap_or_else(|error| panic!("{toolkit:?}: {error}"));
        assert_eq!(bench.text_of(&field), "hello", "{toolkit:?}");
        assert!(verified(&result).contains(&Some(true)), "{toolkit:?}");
        bench
            .act(&field, Action::Clear)
            .unwrap_or_else(|error| panic!("{toolkit:?}: {error}"));
        assert_eq!(bench.text_of(&field), "", "{toolkit:?}");
    });
}

#[test]
#[ignore = "needs lane A's write engine and lane B's caret read; run through scripts/private-session.sh --wm"]
fn type_inserts_ascii_emoji_and_a_combining_mark_at_the_caret() {
    each_toolkit(|toolkit, bench| {
        let field = bench.named("field");
        bench
            .act(&field, Action::SetValue(String::new()))
            .unwrap_or_else(|error| panic!("{toolkit:?}: {error}"));
        for piece in ["ab", "\u{1F600}", "e\u{301}"] {
            bench
                .act(&field, Action::TypeText(piece.to_owned()))
                .unwrap_or_else(|error| panic!("{toolkit:?} {piece:?}: {error}"));
        }
        let read = bench.text_of(&field);
        assert!(read.contains('\u{1F600}'), "{toolkit:?}: {read:?}");
        assert!(read.contains("e\u{301}"), "{toolkit:?}: {read:?}");
        assert!(read.contains("ab"), "{toolkit:?}: {read:?}");
    });
}

#[test]
#[ignore = "needs lane A's write engine; run through scripts/private-session.sh --wm"]
fn a_password_field_write_lands_and_the_result_never_carries_the_text() {
    const SECRET: &str = "zx-secret-7731";
    each_toolkit(|toolkit, bench| {
        let secret = bench.named("secret");
        let outcome = bench.act(&secret, Action::SetValue(SECRET.to_owned()));
        let rendered = match &outcome {
            Ok(result) => serde_json::to_string(result).expect("a result serializes"),
            Err(error) => format!("{error:?}"),
        };
        assert!(!rendered.contains(SECRET), "{toolkit:?}: {rendered}");
        if let Ok(result) = &outcome {
            assert!(!verified(result).contains(&Some(true)), "{toolkit:?}");
        }
    });
}
