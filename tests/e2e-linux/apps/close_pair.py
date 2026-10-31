"""Two native Wayland GTK3 windows of one process, for close-app.

The process quits only once both windows are gone, so its exit shows that a
close request reached every window, not just the first.
"""

import os

os.environ["GTK_USE_PORTAL"] = "0"
os.environ["GDK_BACKEND"] = "wayland"
import gi

gi.require_version("Gtk", "3.0")
from gi.repository import GLib, Gtk

TITLES = ("AD Close One", "AD Close Two")


def main():
    GLib.set_prgname("close_pair.py")
    GLib.set_application_name("Close Pair")
    open_windows = set()

    def closed(window):
        open_windows.discard(window)
        if not open_windows:
            Gtk.main_quit()

    for title in TITLES:
        window = Gtk.Window(title=title)
        window.set_default_size(360, 240)
        window.add(Gtk.Label(label=title))
        window.connect("destroy", closed)
        open_windows.add(window)
        window.show_all()
    Gtk.main()


if __name__ == "__main__":
    main()
