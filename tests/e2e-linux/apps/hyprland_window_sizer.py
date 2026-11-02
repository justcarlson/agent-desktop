"""An undecorated GTK3 window that logs every size and state the compositor gives it, for the
Hyprland window legs.

argv[1] is the JSON-lines log. A `configure` line carries `w` and `h`, the window's logical size
from `Gtk.Window.get_size`, which for an undecorated window is the xdg geometry `j/clients`
reports. A `state` line carries `maximized`, `fullscreen` and `focused`, the toplevel states the
compositor sent. Every line has `t`, monotonic seconds. A native Wayland client is never told
its position, so the log proves size and state only; `j/clients` proves the position.
"""

import json
import os
import sys
import time

os.environ["GTK_USE_PORTAL"] = "0"
os.environ.setdefault("GDK_BACKEND", "wayland")
import gi

gi.require_version("Gtk", "3.0")
gi.require_version("Gdk", "3.0")
from gi.repository import Gdk, GLib, Gtk

SIZER_CLASS = "ad-window-sizer"
SIZER_TITLE = "AD Window Sizer"
STATES = {"maximized": Gdk.WindowState.MAXIMIZED, "fullscreen": Gdk.WindowState.FULLSCREEN,
          "focused": Gdk.WindowState.FOCUSED}


def main(argv):
    if len(argv) != 1:
        raise ValueError("expected the JSON-lines log path")
    stream = open(argv[0], "a", buffering=1)
    GLib.set_prgname(SIZER_CLASS)
    GLib.set_application_name(SIZER_TITLE)
    window = Gtk.Window(title=SIZER_TITLE)
    window.set_decorated(False)
    window.set_default_size(420, 300)
    window.add(Gtk.Label(label=SIZER_TITLE))

    def record(**fields):
        stream.write(json.dumps({**fields, "t": time.monotonic()}) + "\n")

    def configured(widget, _event):
        width, height = widget.get_size()
        record(ev="configure", w=width, h=height)
        return False

    def changed(_widget, event):
        state = event.new_window_state
        record(ev="state", **{name: bool(state & flag) for name, flag in STATES.items()})
        return False

    window.connect("configure-event", configured)
    window.connect("window-state-event", changed)
    window.connect("destroy", lambda *_: Gtk.main_quit())
    window.show_all()
    Gtk.main()


if __name__ == "__main__":
    main(sys.argv[1:])
