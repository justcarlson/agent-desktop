"""A GTK4 window whose same-client popup covers its target button, for the Hyprland hover legs.

It is the L45-3 popup case: on a nested instance's XWayland display (`GDK_BACKEND=x11`), `j/clients`
lists the popup as a second client of the same pid. argv[1] is the JSON-lines log. Each line has `ev`
and `t` (monotonic seconds): `mapped` with the pid, `popup-shown` once the popup is up, and `enter`
with the `widget` the pointer entered. 300 ms after the window maps, "Probe Opener" opens a popup
that does not hide itself, and its "Probe Popup Item" covers the centre of "Probe Target".
"""

import json
import os
import sys
import time

os.environ["GTK_USE_PORTAL"] = "0"
os.environ.setdefault("GDK_BACKEND", "x11")
import gi

gi.require_version("Gtk", "4.0")
from gi.repository import GLib, Gtk

POPUP_CLASS = "ad-hover-popup"
POPUP_TITLE = "AD Hover Popup"
OPEN_AFTER_MS = 300


def main(argv):
    if len(argv) != 1:
        raise ValueError("expected the JSON-lines log path")
    stream = open(argv[0], "a", buffering=1)

    def record(**fields):
        stream.write(json.dumps({**fields, "t": time.monotonic()}) + "\n")

    GLib.set_prgname(POPUP_CLASS)
    GLib.set_application_name(POPUP_TITLE)
    window = Gtk.Window(title=POPUP_TITLE)
    window.set_default_size(560, 420)
    box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)

    def watch(widget, label):
        motion = Gtk.EventControllerMotion()
        motion.connect("enter", lambda *_: record(ev="enter", widget=label))
        widget.add_controller(motion)
        return widget

    def button(label, size):
        widget = Gtk.Button(label=label)
        widget.set_size_request(*size)
        return watch(widget, label)

    opener = watch(Gtk.MenuButton(label="Probe Opener"), "Probe Opener")
    for widget in (opener, button("Probe Target", (320, 140)), button("Probe Other", (320, 60))):
        box.append(widget)
    window.set_child(box)
    popover = Gtk.Popover()
    popover.set_child(button("Probe Popup Item", (360, 300)))
    popover.set_position(Gtk.PositionType.BOTTOM)
    popover.set_autohide(False)
    opener.set_popover(popover)
    popover.connect("map", lambda *_: GLib.timeout_add(OPEN_AFTER_MS, lambda: record(ev="popup-shown") and False))

    def mapped():
        record(ev="mapped", pid=os.getpid())
        GLib.timeout_add(OPEN_AFTER_MS, lambda: opener.popup() and False)
        return False

    window.connect("map", lambda *_: GLib.idle_add(mapped) and False)
    loop = GLib.MainLoop()
    window.connect("close-request", lambda *_: loop.quit() or False)
    window.present()
    loop.run()


if __name__ == "__main__":
    main(sys.argv[1:])
