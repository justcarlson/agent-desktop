"""A GTK3 window that can keep its frame from reporting ACTIVE while the compositor focuses it.

argv[1] is the log path and argv[2] the flag path. While the flag file exists,
the window's focus-in handler stops the signal before GTK's own handler, so
GTK never marks the window active and its accessible frame never gains ACTIVE,
though the compositor made it the active window. Without the flag the window
behaves like any other. Each log line is one JSON object: a key press or
release with its keysym, a button press, a focus change, or the entry's text.
"""

import json
import os
import sys

os.environ["GTK_USE_PORTAL"] = "0"
os.environ["GDK_BACKEND"] = "wayland"
import gi

gi.require_version("Gdk", "3.0")
gi.require_version("Gtk", "3.0")
from gi.repository import Gdk, GLib, Gtk

APP_CLASS = "ad-inactive-frame"
TITLE = "AD Inactive Frame"


def main():
    log, flag = sys.argv[1], sys.argv[2]

    def note(record):
        with open(log, "a") as handle:
            handle.write(json.dumps(record) + "\n")

    def key(kind):
        def handler(_widget, event):
            note({"event": kind, "keysym": Gdk.keyval_name(event.keyval), "state": int(event.state)})
            return False
        return handler

    def focus_in(_widget, _event):
        withheld = os.path.exists(flag)
        note({"event": "focus", "in": True, "withheld": withheld})
        return withheld

    GLib.set_prgname(APP_CLASS)
    GLib.set_application_name("Inactive Frame")
    window = Gtk.Window(title=TITLE)
    window.set_default_size(480, 120)
    entry = Gtk.Entry()
    entry.get_accessible().set_name("inactive frame entry")
    window.add(entry)
    window.connect("key-press-event", key("press"))
    window.connect("key-release-event", key("release"))
    entry.connect("button-press-event", lambda *_: note({"event": "button"}) or False)
    window.connect("focus-in-event", focus_in)
    window.connect("focus-out-event", lambda *_: note({"event": "focus", "in": False}) or False)
    entry.connect("changed", lambda widget: note({"event": "text", "text": widget.get_text()}))
    window.connect("destroy", Gtk.main_quit)
    window.show_all()
    Gtk.main()


if __name__ == "__main__":
    main()
