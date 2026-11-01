"""A GTK3 window that writes every pointer event it receives to a log, for the Hyprland pointer legs.

argv[1] is the JSON-lines log. Each line has `ev`, `widget` and `t` (monotonic seconds).
The window holds a button named "Pad Button" and an event area named "Pad Area".
Presses carry `n` (1, 2 or 3: GTK's single, double and triple press), the button and the
point in window-local logical pixels (`wx`, `wy`). Wheel events carry the direction.
The area also logs motion, so a drag shows its press, its path and its release.
"""

import json
import os
import sys
import time

os.environ["GTK_USE_PORTAL"] = "0"
os.environ["GDK_BACKEND"] = "wayland"
import gi

gi.require_version("Gtk", "3.0")
gi.require_version("Gdk", "3.0")
from gi.repository import Gdk, GLib, Gtk

PAD_CLASS = "ad-pointer-pad"
PAD_TITLE = "AD Pointer Pad"
PRESS_COUNTS = {Gdk.EventType.BUTTON_PRESS: 1, Gdk.EventType._2BUTTON_PRESS: 2, Gdk.EventType._3BUTTON_PRESS: 3}
MASK = (Gdk.EventMask.BUTTON_PRESS_MASK | Gdk.EventMask.BUTTON_RELEASE_MASK | Gdk.EventMask.SCROLL_MASK
        | Gdk.EventMask.SMOOTH_SCROLL_MASK)


def main(argv):
    if len(argv) != 1:
        raise ValueError("expected the JSON-lines log path")
    stream = open(argv[0], "a", buffering=1)
    GLib.set_prgname(PAD_CLASS)
    GLib.set_application_name(PAD_TITLE)
    window = Gtk.Window(title=PAD_TITLE)
    window.set_default_size(640, 480)
    box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
    window.add(box)

    def record(**fields):
        stream.write(json.dumps({**fields, "t": time.monotonic()}) + "\n")

    def local(widget, event):
        x, y = widget.translate_coordinates(window, int(event.x), int(event.y))
        return {"wx": x, "wy": y}

    def watch(widget, name):
        widget.add_events(MASK)
        widget.connect("button-press-event", lambda w, e: record(
            ev="press", widget=name, n=PRESS_COUNTS.get(e.type, 0), button=e.button, **local(w, e)) and False)
        widget.connect("button-release-event", lambda w, e: record(
            ev="release", widget=name, button=e.button, **local(w, e)) and False)
        widget.connect("scroll-event", lambda w, e: scrolled(name, e) and False)

    def scrolled(name, event):
        found, dx, dy = event.get_scroll_deltas()
        record(ev="scroll", widget=name, direction=event.direction.value_nick,
               dx=dx if found else None, dy=dy if found else None)

    button = Gtk.Button(label="Pad Button")
    button.get_accessible().set_name("Pad Button")
    button.set_size_request(320, 120)
    watch(button, "button")
    box.pack_start(button, False, False, 0)
    area = Gtk.EventBox()
    area.get_accessible().set_name("Pad Area")
    area.set_size_request(320, 240)
    watch(area, "area")
    area.add_events(Gdk.EventMask.POINTER_MOTION_MASK)
    area.connect("motion-notify-event", lambda w, e: record(ev="motion", widget="area", **local(w, e)) and False)
    box.pack_start(area, True, True, 0)
    window.connect("destroy", lambda *_: Gtk.main_quit())
    window.show_all()
    Gtk.main()


if __name__ == "__main__":
    main(sys.argv[1:])
