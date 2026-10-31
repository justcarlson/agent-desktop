"""Two GTK3 windows of one process with the same title.

A leg puts each alone on its own workspace, so they also share one size.
Nothing in the compositor's evidence then tells them apart, so a focus that
binds by evidence must refuse them as ambiguous.
"""

import os

os.environ["GTK_USE_PORTAL"] = "0"
import gi

gi.require_version("Gtk", "3.0")
from gi.repository import GLib, Gtk

TITLE = "AD Twin"


def main():
    GLib.set_prgname("twin_windows.py")
    GLib.set_application_name("Twin Windows")
    windows = []
    for index in range(2):
        window = Gtk.Window(title=TITLE)
        window.set_default_size(400, 300)
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        box.add(Gtk.Label(label=f"twin {index}"))
        box.add(Gtk.Entry())
        window.add(box)
        window.connect("destroy", Gtk.main_quit)
        window.show_all()
        windows.append(window)
    Gtk.main()


if __name__ == "__main__":
    main()
