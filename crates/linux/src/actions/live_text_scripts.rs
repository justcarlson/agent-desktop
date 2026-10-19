pub(super) const GTK3_SCRIPT: &str = r#"
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

pub(super) const GTK4_SCRIPT: &str = r#"
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

pub(super) const QT_SCRIPT: &str = r#"
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
