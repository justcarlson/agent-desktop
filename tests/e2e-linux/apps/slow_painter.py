"""A GTK 4 window that redraws slowly and logs what it drew.

`slow_painter.py APP_ID DELAY_MS LOG` maps one window of class and title
APP_ID and paints tick `n` as the ticker colour of terminal_painter. Each
frame-clock tick advances the tick and asks for a draw; the draw sleeps
DELAY_MS before it paints, so the commit lands DELAY_MS after the compositor
asked for a frame. A window the compositor does not show gets no frame-clock
ticks and keeps its last tick. Before each paint it writes
`"<n> <CLOCK_MONOTONIC ns>"` to LOG, so a tick logged after a request was
drawn after it.
"""

import os
import sys
import time

os.environ["GTK_USE_PORTAL"] = "0"
os.environ["GSK_RENDERER"] = "cairo"
import gi

gi.require_version("Gtk", "4.0")
from gi.repository import GLib, Gtk  # noqa: E402

TICK_BLUE = 0x40


def tick_colour(tick: int) -> tuple[float, float, float]:
    return (tick % 256) / 255, ((tick // 256) % 256) / 255, TICK_BLUE / 255


def main(argv: list[str]) -> None:
    if len(argv) != 3:
        raise ValueError("expected APP_ID DELAY_MS LOG")
    app_id, delay, log = argv[0], int(argv[1]) / 1000, argv[2]
    GLib.set_prgname(app_id)
    Gtk.init()
    loop = GLib.MainLoop()
    state = {"tick": 1}
    out = open(log, "a", buffering=1)

    def draw(_area, cr, _width, _height):
        time.sleep(delay)
        tick = state["tick"]
        out.write(f"{tick} {time.monotonic_ns()}\n")
        cr.set_source_rgb(*tick_colour(tick))
        cr.paint()

    def advance(area, _clock):
        state["tick"] += 1
        area.queue_draw()
        return GLib.SOURCE_CONTINUE

    window = Gtk.Window(title=app_id)
    window.set_default_size(320, 240)
    area = Gtk.DrawingArea()
    area.set_draw_func(draw)
    area.add_tick_callback(advance)
    window.set_child(area)
    window.connect("close-request", lambda _window: loop.quit() or False)
    window.present()
    loop.run()


if __name__ == "__main__":
    main(sys.argv[1:])
