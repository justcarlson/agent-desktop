"""Probe: does pointer motion to a ref's point on a Hyprland client enter that element?

Run only inside an Omarchy VM session (test/omarchy-vm run), never on a host:

    python3 -I probes/linux/hyprland-hover-receiver.py run ~/out/hover-probe

A headed hover by ref is one virtual-pointer motion to the ref's centre. This probe sends that motion
with `--headed mouse-move --xy` to the product's own bounds of "Probe Target" and reads the fixture's
own enter log, which is the effect oracle. It uses the fixtures and cases of
`tests/e2e-linux/hyprland_hit_probe.py`, on its nested instance (native Wayland clients) and on that
instance's XWayland display. Cases: plain, overlay (an in-window "Probe Shield" over the content) and
popup (a same-client, non-grabbing popup over the target centre).

Per run it records the evidence a receiver proof could read before dispatch (the compositor's clients
for the fixture pid and the AT-SPI `GetAccessibleAtPoint` descent at the point), the product's current
`--headed hover` answer on the ref, the enter events after the motion, and, as the last step, the
product's `--headed click` answer with the fixture's click log. Results go to `<out>/results.jsonl` and
a summary table to stdout.
"""

from __future__ import annotations

import json
import os
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tests" / "e2e-linux"))

import hyprland_hit_probe as hit  # noqa: E402

TOOLKITS = tuple(os.environ.get("PROBE_TOOLKITS", "gtk3,gtk4,qt6").split(","))
CASES = ("plain", "overlay", "popup")
ROUNDS = 2
SETTLE = 1.5


def fixture(toolkit: str, cover: str) -> None:
    import time

    def emit(**fields):
        print(json.dumps({**fields, "t": time.monotonic()}), flush=True)

    if toolkit == "qt6":
        return qt_fixture(cover, emit)
    os.environ.setdefault("GDK_BACKEND", "wayland")
    os.environ["GTK_USE_PORTAL"] = "0"
    import gi
    gi.require_version("Gtk", "3.0" if toolkit == "gtk3" else "4.0")
    from gi.repository import GLib, Gtk
    gtk3 = toolkit == "gtk3"
    window = Gtk.Window(title=hit.TITLE)
    window.set_default_size(560, 420)
    box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)

    def watch(widget, label):
        if isinstance(widget, Gtk.Button):
            widget.connect("clicked", lambda *_: emit(ev="clicked", widget=label))
        if gtk3:
            widget.connect("enter-notify-event", lambda *_: emit(ev="enter", widget=label) and False)
        else:
            motion = Gtk.EventControllerMotion()
            motion.connect("enter", lambda *_: emit(ev="enter", widget=label))
            widget.add_controller(motion)
        return widget

    def button(label, size):
        widget = Gtk.Button(label=label)
        widget.set_size_request(*size)
        return watch(widget, label)

    opener = watch(Gtk.MenuButton(label="Probe Opener"), "Probe Opener")
    target, other = button("Probe Target", (320, 140)), button("Probe Other", (320, 60))
    for widget in (opener, target, other):
        box.pack_start(widget, False, False, 0) if gtk3 else box.append(widget)
    overlay = Gtk.Overlay()
    overlay.add(box) if gtk3 else overlay.set_child(box)
    if cover == "overlay":
        overlay.add_overlay(button("Probe Shield", (1, 1)))
    window.add(overlay) if gtk3 else window.set_child(overlay)
    if cover == "popup":
        popover = Gtk.Popover()
        item = button("Probe Popup Item", (360, 300))
        popover.add(item) if gtk3 else popover.set_child(item)
        popover.set_position(Gtk.PositionType.BOTTOM)
        opener.set_popover(popover)
        if gtk3:
            popover.set_modal(False)
            popover.set_constrain_to(Gtk.PopoverConstraint.NONE)
            item.show()
        else:
            popover.set_autohide(False)
        popover.connect("map", lambda *_: GLib.timeout_add(300, lambda: emit(ev="popup-shown") and False))
        popover.connect("closed", lambda *_: emit(ev="popup-closed"))

    def mapped(*_):
        emit(ev="mapped", pid=os.getpid())
        if cover == "popup":
            GLib.timeout_add(300, lambda: (opener.set_active(True) if gtk3 else opener.popup()) and False)
        return False

    window.connect("map-event" if gtk3 else "map", lambda *_: GLib.idle_add(mapped) and False)
    if gtk3:
        window.connect("destroy", Gtk.main_quit)
        window.show_all()
        Gtk.main()
    else:
        loop = GLib.MainLoop()
        window.connect("close-request", lambda *_: loop.quit() or False)
        window.present()
        loop.run()


def qt_fixture(cover: str, emit) -> None:
    os.environ.setdefault("QT_QPA_PLATFORM", "wayland")
    os.environ["QT_ACCESSIBILITY"] = os.environ["QT_LINUX_ACCESSIBILITY_ALWAYS_ON"] = "1"
    from PyQt6.QtCore import QEvent, QObject, QPoint, Qt, QTimer
    from PyQt6.QtWidgets import QApplication, QPushButton, QStackedLayout, QVBoxLayout, QWidget

    class Enters(QObject):
        def eventFilter(self, watched, event):
            if event.type() == QEvent.Type.Enter and isinstance(watched, QPushButton):
                emit(ev="enter", widget=watched.accessibleName())
            return False

    app = QApplication([sys.argv[0]])
    enters = Enters()
    window = QWidget()
    window.setWindowTitle(hit.TITLE)
    window.resize(560, 420)
    stack = QStackedLayout(window)
    stack.setStackingMode(QStackedLayout.StackingMode.StackAll)
    content = QWidget()
    layout = QVBoxLayout(content)

    def button(label, parent=None, height=60):
        widget = QPushButton(label, parent)
        widget.setAccessibleName(label)
        widget.setMinimumHeight(height)
        widget.clicked.connect(lambda: emit(ev="clicked", widget=label))
        widget.installEventFilter(enters)
        return widget

    opener, target = button("Probe Opener", height=40), button("Probe Target", height=140)
    for widget in (opener, target, button("Probe Other")):
        layout.addWidget(widget)
    layout.addStretch()
    stack.addWidget(content)
    if cover == "overlay":
        shield = button("Probe Shield")
        stack.addWidget(shield)
        stack.setCurrentWidget(shield)
    popup = QWidget(window, Qt.WindowType.ToolTip)
    popup_layout = QVBoxLayout(popup)
    popup_layout.addWidget(button("Probe Popup Item", height=280))

    def open_popup():
        popup.resize(360, 300)
        popup.move(opener.mapToGlobal(QPoint(0, opener.height())))
        popup.show()
        QTimer.singleShot(300, lambda: emit(ev="popup-shown"))

    def mapped():
        emit(ev="mapped", pid=os.getpid())
        if cover == "popup":
            QTimer.singleShot(300, open_popup)

    window.show()
    QTimer.singleShot(300, mapped)
    app.exec()


def centre(bounds) -> tuple[int, int] | None:
    if not isinstance(bounds, dict):
        return None
    rect = bounds.get("value", bounds)
    try:
        return round(rect["x"] + rect["width"] / 2), round(rect["y"] + rect["height"] / 2)
    except (KeyError, TypeError):
        return None


def one_run(binary: str, env: dict[str, str], hypr, atspi, toolkit: str, case: str) -> dict:
    import subprocess
    app = subprocess.Popen([sys.executable, "-I", __file__, "fixture", toolkit, case], env=env,
                           stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
    lines = hit.Lines(app.stdout)
    result: dict = {"toolkit": toolkit, "case": case}
    try:
        lines.until(lambda seen: any(item.get("ev") == "mapped" for item in seen), 15)
        hypr.wait_event("openwindow", hit.TITLE, 5, lambda: hypr.client(app.pid) is not None)
        client = hypr.client(app.pid)
        if client is None:
            return {**result, "verdict": "setup_failed", "why": "fixture never mapped"}
        result["fixture_client"] = {key: client.get(key) for key in ("at", "size", "xwayland")}
        if case == "popup":
            lines.until(lambda seen: any(item.get("ev") == "popup-shown" for item in seen), 5)
            result["popup_shown"] = any(item.get("ev") == "popup-shown" for item in lines.seen)
        ref = hit.target_ref(binary, env, app.pid)
        if ref is None:
            return {**result, "verdict": "setup_failed", "why": "no unique ref for Probe Target"}
        result["product_bounds"] = hit.product(binary, env, "get", ref, "--property", "bounds").get("data")
        point = centre(result["product_bounds"])
        if point is None:
            return {**result, "verdict": "setup_failed", "why": "no product bounds"}
        result["point"] = point
        result["stack_clients"] = sum(1 for item in hypr.query("clients") if item.get("pid") == app.pid)
        root = atspi.app(app.pid)
        frames = [node for node in atspi.children(root) if atspi.describe(node)["name"] == hit.TITLE] if root else []
        target = atspi.named(root, "Probe Target") if root else None
        if target and frames:
            wx, wy, ww, wh = atspi.describe(target).get("window", (0, 0, 0, 0))
            chain = atspi.descend(frames[0], (wx + ww // 2, wy + wh // 2), 1)
            result["descent"] = [item.get("name") for item in chain]
        (x, y), (width, height) = client["at"], client["size"]
        park = (x + width - 8, y + height - 8)
        result["park"] = hit.product(binary, env, "--headed", "mouse-move", f"--xy={park[0]},{park[1]}").get("ok")
        lines.until(lambda _: False, 0.4)
        mark = len(lines.seen)
        result["hover_ref"] = hit.product(binary, env, "--headed", "hover", ref)
        after_hover = lines.until(lambda _: False, 0.6)[mark:]
        result["hover_ref_events"] = [item.get("widget") for item in after_hover if item.get("ev") == "enter"]
        mark = len(lines.seen)
        result["move"] = hit.product(binary, env, "--headed", "mouse-move", f"--xy={point[0]},{point[1]}")
        result["cursorpos"] = hypr.query("cursorpos")
        seen = lines.until(lambda all_: any(item.get("ev") == "enter" for item in all_[mark:]), SETTLE)[mark:]
        lines.until(lambda _: False, 0.4)
        result["enters"] = [item.get("widget") for item in lines.seen[mark:] if item.get("ev") == "enter"]
        entered = result["enters"]
        result["hover_verdict"] = ("reaches" if entered and entered[-1] == "Probe Target"
                                   else f"intercepted:{entered[-1]}" if entered else "no_enter")
        mark = len(lines.seen)
        result["click"] = hit.product(binary, env, "--headed", "click", ref)
        settle = SETTLE if result["click"].get("ok") else 0.8
        seen = lines.until(lambda all_: any(item.get("ev") == "clicked" for item in all_[mark:]), settle)[mark:]
        clicked = [item.get("widget") for item in seen if item.get("ev") == "clicked"]
        result["clicked"] = clicked
        ok = bool(result["click"].get("ok"))
        result["click_verdict"] = (("hit" if clicked == ["Probe Target"] else "false_success") if ok
                                   else ("refused" if not clicked else "refused_with_effect"))
        return result
    finally:
        hit.stop(app)


def matrix(binary: str, env: dict[str, str], surface: str, stream) -> list[dict]:
    hypr, atspi, rows = hit.Hypr(env), hit.Atspi(), []
    for toolkit in TOOLKITS:
        for case in CASES:
            for round_ in range(ROUNDS):
                try:
                    row = one_run(binary, env, hypr, atspi, toolkit, case)
                except Exception as error:
                    row = {"toolkit": toolkit, "case": case, "verdict": "probe_error", "why": repr(error)[:300]}
                row.update(round=round_, surface=surface)
                stream.write(json.dumps(row) + "\n")
                stream.flush()
                rows.append(row)
    return rows


def cause(answer: dict) -> str:
    error = answer.get("error") or {}
    details = error.get("details") or {}
    return f"{error.get('code', 'ok')}/{details.get('cause', '-')}"


def run(out: Path) -> int:
    if not os.environ.get("OMARCHY_VM_CHANNEL"):
        sys.exit("hyprland-hover-receiver: run this inside an Omarchy VM, never on a host")
    out.mkdir(parents=True, exist_ok=True)
    home = tempfile.mkdtemp(prefix="ad-hover-probe-")
    env = dict(os.environ, AGENT_DESKTOP_HOME=home, GDK_BACKEND="wayland", QT_QPA_PLATFORM="wayland",
               QT_ACCESSIBILITY="1", QT_LINUX_ACCESSIBILITY_ALWAYS_ON="1", GTK_USE_PORTAL="0")
    binary = os.environ.get("AGENT_DESKTOP_E2E_BIN", str(Path.home() / "src/agent-desktop-bin/agent-desktop"))
    rows = []
    with (out / "results.jsonl").open("w") as stream:
        nested, bound = hit.start_nested(env, out)
        try:
            display = bound.pop("PROBE_XWAYLAND_DISPLAY", "")
            rows += matrix(binary, bound, "nested", stream)
            if display:
                xwayland = dict(bound, DISPLAY=display, GDK_BACKEND="x11", QT_QPA_PLATFORM="xcb")
                rows += matrix(binary, xwayland, "xwayl", stream)
            else:
                rows.append({"surface": "xwayl", "toolkit": "-", "case": "-", "round": 0,
                             "verdict": "setup_failed", "why": "the nested instance served no X display"})
        finally:
            hit.stop(nested)
    for row in rows:
        if "hover_verdict" not in row:
            print(f"{row['surface']:6} {row['toolkit']:5} {row['case']:8} r{row['round']} "
                  f"{row.get('verdict')} {row.get('why')}")
            continue
        print(f"{row['surface']:6} {row['toolkit']:5} {row['case']:8} r{row['round']} "
              f"hover={row['hover_verdict']:28} enters={row['enters']} move={cause(row['move'])} "
              f"hover_ref={cause(row['hover_ref'])} hover_ref_events={row['hover_ref_events']} "
              f"stack={row['stack_clients']} descent={(row.get('descent') or ['-'])[-1]} "
              f"click={row['click_verdict']}:{cause(row['click'])} clicked={row['clicked']}")
    return 0


if __name__ == "__main__":
    if sys.argv[1:2] == ["fixture"]:
        fixture(sys.argv[2], sys.argv[3])
    elif sys.argv[1:2] == ["run"] and len(sys.argv) == 3:
        sys.exit(run(Path(sys.argv[2]).expanduser()))
    else:
        sys.exit(__doc__)
