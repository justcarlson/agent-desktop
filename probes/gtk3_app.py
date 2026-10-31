import gi; gi.require_version("Gtk","3.0")
from gi.repository import Gtk
w=Gtk.Window(title="ADTest"); b=Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
lab=Gtk.Label(label="count 0"); n=[0]
def inc(_): n[0]+=1; lab.set_text(f"count {n[0]}")
btn=Gtk.Button(label="Increment"); btn.connect("clicked",inc)
e=Gtk.Entry(); e.set_name("field"); chk=Gtk.CheckButton(label="Opt")
for x in (lab,btn,e,chk): b.add(x)
w.add(b); w.connect("destroy",Gtk.main_quit); w.show_all(); Gtk.main()
