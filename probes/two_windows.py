import gi, sys; gi.require_version("Gtk","3.0")
from gi.repository import Gtk
title = sys.argv[1] if len(sys.argv) > 1 else "ADTwin"
for i, (w, h) in enumerate(((400, 300), (400, 300) if "--same-size" in sys.argv else (500, 350))):
    win = Gtk.Window(title=title); win.set_default_size(w, h)
    b = Gtk.Box(orientation=Gtk.Orientation.VERTICAL); lab = Gtk.Label(label=f"twin {i}")
    b.add(lab); b.add(Gtk.Entry()); win.add(b); win.connect("destroy", Gtk.main_quit); win.show_all()
Gtk.main()
