"""Two native Wayland GTK3 windows of one process with the same title.

The compositor sizes each window. When both get one size, nothing in the
compositor's evidence tells them apart; when their sizes differ, the size
does.
"""

import os

os.environ["GTK_USE_PORTAL"] = "0"
os.environ["GDK_BACKEND"] = "wayland"
import gi

gi.require_version("Gtk", "3.0")
from gi.repository import GLib, Gtk

TITLE = "AD Twin"


def main():
    GLib.set_prgname("twin_windows.py")
    GLib.set_application_name("Twin Windows")
    for index in range(2):
        window = Gtk.Window(title=TITLE)
        window.set_default_size(400, 300)
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        box.add(Gtk.Label(label=f"twin {index}"))
        box.add(Gtk.Entry())
        window.add(box)
        window.connect("destroy", Gtk.main_quit)
        window.show_all()
    Gtk.main()


if __name__ == "__main__":
    main()
