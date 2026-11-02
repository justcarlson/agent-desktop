"""A GTK3 window with one entry that writes every key event it receives to a log.

argv[1] is the log path. With argv[2] set to `opaque`, the window shows a
drawing area that reports the entry role but has no Text or EditableText
interface in place of the entry, so a type into it can only be physical keys.
Each line is one JSON object: a key press or release
with its keysym name, hardware keycode, modifier state and XKB layout group, a
focus change of the window, or the entry's whole text after it changed. A leg
reads the log and the entry's accessible text, never the command's own answer.
"""

import json
import os
import sys

os.environ["GTK_USE_PORTAL"] = "0"
os.environ["GDK_BACKEND"] = "wayland"
import gi

gi.require_version("Atk", "1.0")
gi.require_version("Gdk", "3.0")
gi.require_version("Gtk", "3.0")
from gi.repository import Atk, Gdk, GLib, Gtk

KEY_LOG_CLASS = "ad-key-log"
TITLE = "AD Key Log"
OPAQUE = "opaque"
OPAQUE_NAME = "opaque key log field"


def main():
    log = sys.argv[1]

    def note(record):
        with open(log, "a") as handle:
            handle.write(json.dumps(record) + "\n")

    def key(kind):
        def handler(_widget, event):
            note({"event": kind, "keysym": Gdk.keyval_name(event.keyval), "code": event.hardware_keycode,
                  "state": int(event.state), "group": event.group})
            return False
        return handler

    GLib.set_prgname(KEY_LOG_CLASS)
    GLib.set_application_name("Key Log")
    window = Gtk.Window(title=TITLE)
    window.set_default_size(480, 120)
    if sys.argv[2:3] == [OPAQUE]:
        field = Gtk.DrawingArea()
        field.set_size_request(200, 30)
        field.set_can_focus(True)
        field.get_accessible().set_role(Atk.Role.ENTRY)
        field.get_accessible().set_name(OPAQUE_NAME)
        window.add(field)
    else:
        entry = Gtk.Entry()
        entry.get_accessible().set_name("key log entry")
        entry.connect("changed", lambda widget: note({"event": "text", "text": widget.get_text()}))
        window.add(entry)
    window.connect("key-press-event", key("press"))
    window.connect("key-release-event", key("release"))
    window.connect("focus-in-event", lambda *_: note({"event": "focus", "in": True}) or False)
    window.connect("focus-out-event", lambda *_: note({"event": "focus", "in": False}) or False)
    window.connect("destroy", Gtk.main_quit)
    window.show_all()
    Gtk.main()


if __name__ == "__main__":
    main()
