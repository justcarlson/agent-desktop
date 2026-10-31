"""Two native Wayland GTK3 windows of one process with one title, for focus-window.

The compositor sizes each window. Alone on equal workspaces they share one
size and nothing tells them apart; once one shares its workspace, the sizes
differ and each window binds to its own toplevel.
"""

import os

os.environ["GTK_USE_PORTAL"] = "0"
os.environ["GDK_BACKEND"] = "wayland"
import gi

gi.require_version("Gtk", "3.0")
from gi.repository import GLib, Gtk

TITLE = "AD Focus Twin"


def main():
    GLib.set_prgname("focus_twins.py")
    GLib.set_application_name("Focus Twins")
    for index in range(2):
        window = Gtk.Window(title=TITLE)
        window.set_default_size(420, 320)
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        box.add(Gtk.Label(label=f"focus twin {index}"))
        box.add(Gtk.Button(label=f"twin button {index}"))
        window.add(box)
        window.connect("destroy", Gtk.main_quit)
        window.show_all()
    Gtk.main()


if __name__ == "__main__":
    main()
