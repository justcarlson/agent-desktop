import gi, os, sys; gi.require_version("Gtk","3.0"); gi.require_version("Gdk","3.0")
from gi.repository import Gtk, Gdk
exec(open(os.environ["APP"]).read().replace("w.show_all(); Gtk.main()", "pass"))
def log(kind):
    def f(win, ev):
        print(kind, Gdk.keyval_name(ev.keyval), ev.hardware_keycode, "focus=", type(win.get_focus()).__name__, flush=True)
        return False
    return f
w.connect("key-press-event", log("press")); w.connect("key-release-event", log("release"))
w.connect("notify::is-active", lambda *_: print("is-active", w.is_active(), flush=True))
e.connect("changed", lambda *_: print("entry", repr(e.get_text()), flush=True))
w.show_all(); Gtk.main()
