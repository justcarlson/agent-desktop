"""A GTK4 application window and a Gtk.Dialog outside its GtkApplication, for close-app.

The dialog is never added to the application, so its app id is the program
name, not the application id, and a toplevel list cannot tie the two by app id.
argv[1] is "parented" (transient for the main window, modal) or "unparented".
Every close request either window receives is written to argv[2], so a leg
can read that none arrived. The dialog maps only after the main window, so
the run's exec rule places the main window and a class rule the dialog.
"""

import os
import sys

os.environ["GTK_USE_PORTAL"] = "0"
os.environ["GDK_BACKEND"] = "wayland"
import gi

gi.require_version("Gtk", "4.0")
from gi.repository import GLib, Gtk

APP_ID = "org.agentdesktop.DialogHost"
DIALOG_CLASS = "ad-dialog-host"
HOST_TITLE = "AD Dialog Host"
DIALOG_TITLE = "AD Unrelated Dialog"
DIALOG_DELAY_MS = 400


def main():
    mode, log = sys.argv[1], sys.argv[2]

    def note(text):
        with open(log, "a") as handle:
            handle.write(text + "\n")

    def show_dialog(window):
        dialog = Gtk.Dialog(title=DIALOG_TITLE)
        if mode == "parented":
            dialog.set_transient_for(window)
            dialog.set_modal(True)
        dialog.set_default_size(320, 160)
        dialog.add_button("OK", Gtk.ResponseType.OK)
        dialog.connect("close-request", lambda _dialog: note("close-request dialog") or False)
        dialog.present()
        return GLib.SOURCE_REMOVE

    def activate(app):
        window = Gtk.ApplicationWindow(application=app, title=HOST_TITLE)
        window.set_default_size(420, 300)
        window.set_child(Gtk.Label(label=HOST_TITLE))
        window.connect("close-request", lambda _window: note("close-request host") or False)
        window.present()
        GLib.timeout_add(DIALOG_DELAY_MS, show_dialog, window)

    GLib.set_prgname(DIALOG_CLASS)
    GLib.set_application_name("Dialog Host")
    app = Gtk.Application(application_id=APP_ID)
    app.connect("activate", activate)
    app.run([sys.argv[0]])


if __name__ == "__main__":
    main()
