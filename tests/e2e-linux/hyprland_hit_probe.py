"""Probe: does a headed ref click on a native Wayland client under Hyprland land on its element?

Run only inside an Omarchy VM session (test/omarchy-vm run), never on a host:

    python3 -I tests/e2e-linux/hyprland_hit_probe.py run ~/out/hit-probe

For each toolkit (GTK3, GTK4, Qt6, all native Wayland) and each case it starts a fixture window,
sets up the case's cover, reads the receiver evidence the product could use before dispatch, sends
`--headed click` on the ref of "Probe Target", and reads the fixture's own click log, which is the
effect oracle. Cases:

- plain: nothing covers the target.
- overlay: an in-window button ("Probe Shield") covers the whole window content.
- popup: a non-grabbing popup surface of the same client, opened from "Probe Opener", covers the
  target's centre (GTK3 popover with constrain-to none, GTK4 popover, Qt tool-tip window).
- layer: a slurp selection layer covers the output.
- float: a floating foot window covers the fixture window.

Pre-dispatch reads per case: the application's showing top-level children, the
`GetAccessibleAtPoint` descent from the frame at the target's window-coordinate centre in both
coordinate types, the product's own bounds of the target, and the compositor's clients and layers.
Results go to `<out>/results.jsonl` and a summary table to stdout.
"""

from __future__ import annotations

import json
import os
import selectors
import socket
import subprocess
import sys
import tempfile
import time
from pathlib import Path

TITLE = "AD Hit Probe"
COVER_CLASS = "ad-hit-probe-cover"
TOOLKITS = ("gtk3", "gtk4", "qt6")
CASES = ("plain", "overlay", "popup", "layer", "float")
ROUNDS = 2
NULL_PATH = "/org/a11y/atspi/null"
COMPONENT = "org.a11y.atspi.Component"
ACCESSIBLE = "org.a11y.atspi.Accessible"


def fixture(toolkit: str, cover: str) -> None:
    def emit(**fields):
        print(json.dumps({**fields, "t": time.monotonic()}), flush=True)

    if toolkit == "qt6":
        return qt_fixture(cover, emit)
    os.environ["GDK_BACKEND"] = "wayland"
    os.environ["GTK_USE_PORTAL"] = "0"
    import gi
    gi.require_version("Gtk", "3.0" if toolkit == "gtk3" else "4.0")
    from gi.repository import GLib, Gtk
    gtk3 = toolkit == "gtk3"
    window = Gtk.Window(title=TITLE)
    window.set_default_size(560, 420)
    box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)

    def button(label, size):
        widget = Gtk.Button(label=label)
        widget.set_size_request(*size)
        widget.connect("clicked", lambda *_: emit(ev="clicked", widget=label))
        return widget

    opener = Gtk.MenuButton(label="Probe Opener")
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
    os.environ["QT_QPA_PLATFORM"] = "wayland"
    os.environ["QT_ACCESSIBILITY"] = os.environ["QT_LINUX_ACCESSIBILITY_ALWAYS_ON"] = "1"
    from PyQt6.QtCore import QPoint, Qt, QTimer
    from PyQt6.QtWidgets import QApplication, QPushButton, QStackedLayout, QVBoxLayout, QWidget
    app = QApplication([sys.argv[0]])
    window = QWidget()
    window.setWindowTitle(TITLE)
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


class Lines:
    """The fixture's stdout, read as JSON lines without polling."""

    def __init__(self, stream):
        self.stream, self.seen = stream, []
        self.selector = selectors.DefaultSelector()
        self.selector.register(stream, selectors.EVENT_READ)

    def until(self, check, timeout: float) -> list[dict]:
        deadline = time.monotonic() + timeout
        while not check(self.seen):
            left = deadline - time.monotonic()
            if left <= 0 or not self.selector.select(left):
                break
            line = self.stream.readline()
            if not line:
                break
            try:
                self.seen.append(json.loads(line))
            except ValueError:
                continue
        return self.seen


class Hypr:
    def __init__(self, env: dict[str, str]):
        self.env = env
        base = Path(env["XDG_RUNTIME_DIR"]) / "hypr" / env["HYPRLAND_INSTANCE_SIGNATURE"]
        self.socket, self.events = base / ".socket.sock", base / ".socket2.sock"
        self.lua = self.ctl("eval", "return 1") == "ok"

    def ctl(self, *argv: str) -> str:
        done = subprocess.run(["hyprctl", *argv], env=self.env, capture_output=True, text=True, timeout=5)
        return (done.stdout + done.stderr).strip()

    def query(self, name: str):
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as client:
            client.settimeout(3)
            client.connect(str(self.socket))
            client.sendall(f"j/{name}".encode())
            chunks = []
            while chunk := client.recv(65536):
                chunks.append(chunk)
        return json.loads(b"".join(chunks))

    def wait_event(self, prefix: str, contains: str, timeout: float, ready=None) -> bool:
        """Waits on the event socket for `prefix>>...contains...`, unless `ready()` already holds."""
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as client:
            client.connect(str(self.events))
            if ready is not None and ready():
                return True
            deadline, buffer = time.monotonic() + timeout, b""
            while (left := deadline - time.monotonic()) > 0:
                client.settimeout(left)
                try:
                    chunk = client.recv(65536)
                except TimeoutError:
                    return False
                buffer += chunk
                *lines, buffer = buffer.split(b"\n")
                if any(line.decode(errors="replace").startswith(prefix + ">>") and contains in
                       line.decode(errors="replace") for line in lines):
                    return True
        return False

    def client(self, pid: int) -> dict | None:
        found = [item for item in self.query("clients") if item.get("pid") == pid and item.get("mapped")]
        return found[0] if len(found) == 1 else None

    def window(self, verb: str, address: str, x: int = 0, y: int = 0) -> str:
        target = f"address:{address}"
        if self.lua:
            fields = (f'action="enable",window="{target}"' if verb == "float"
                      else f'x={x},y={y},window="{target}"')
            return self.ctl("dispatch", f"hl.dsp.window.{verb}({{{fields}}})")
        if verb == "float":
            return self.ctl("dispatch", "setfloating", target)
        return self.ctl("dispatch", f"{verb}windowpixel", f"exact {x} {y},{target}")


class Atspi:
    def __init__(self):
        from gi.repository import Gio, GLib
        self.Gio, self.GLib = Gio, GLib
        session = Gio.bus_get_sync(Gio.BusType.SESSION, None)
        address = session.call_sync("org.a11y.Bus", "/org/a11y/bus", "org.a11y.Bus", "GetAddress", None,
                                    None, Gio.DBusCallFlags.NONE, 2000, None).unpack()[0]
        flags = Gio.DBusConnectionFlags.AUTHENTICATION_CLIENT | Gio.DBusConnectionFlags.MESSAGE_BUS_CONNECTION
        self.bus = Gio.DBusConnection.new_for_address_sync(address, flags, None, None)

    def call(self, node, interface, method, signature=None, args=()):
        variant = self.GLib.Variant(signature, args) if signature else None
        return self.bus.call_sync(node[0], node[1], interface, method, variant, None,
                                  self.Gio.DBusCallFlags.NONE, 1500, None).unpack()

    def pid(self, name: str) -> int:
        return self.call(("org.freedesktop.DBus", "/org/freedesktop/DBus"), "org.freedesktop.DBus",
                         "GetConnectionUnixProcessID", "(s)", (name,))[0]

    def app(self, pid: int):
        for name, path in self.children(("org.a11y.atspi.Registry", "/org/a11y/atspi/accessible/root")):
            try:
                if self.pid(name) == pid:
                    return name, path
            except Exception:
                continue
        return None

    def children(self, node):
        return [(bus or node[0], path) for bus, path in self.call(node, ACCESSIBLE, "GetChildren")[0]
                if path != NULL_PATH]

    def describe(self, node) -> dict:
        states = self.call(node, ACCESSIBLE, "GetState")[0]
        out = {"role": self.call(node, ACCESSIBLE, "GetRoleName")[0],
               "name": self.call(node, "org.freedesktop.DBus.Properties", "Get", "(ss)", (ACCESSIBLE, "Name"))[0],
               "showing": bool(states[0] & (1 << 25)), "path": node[1]}
        try:
            out["screen"] = list(self.call(node, COMPONENT, "GetExtents", "(u)", (0,))[0])
            out["window"] = list(self.call(node, COMPONENT, "GetExtents", "(u)", (1,))[0])
        except Exception as error:
            out["extents_error"] = str(error)[:120]
        return out

    def named(self, root, name: str):
        pending = [root]
        while pending:
            node = pending.pop()
            if self.describe(node)["name"] == name:
                return node
            pending.extend(self.children(node))
        return None

    def descend(self, frame, point: tuple[int, int], coord: int) -> list[dict]:
        chain, node = [], frame
        for _ in range(50):
            try:
                bus, path = self.call(node, COMPONENT, "GetAccessibleAtPoint", "(iiu)", (*point, coord))[0]
            except Exception as error:
                chain.append({"error": str(error)[:120]})
                break
            found = (bus or node[0], path)
            if path == NULL_PATH or found == node:
                break
            chain.append(self.describe(found))
            node = found
        return chain


def product(binary: str, env: dict[str, str], *argv: str) -> dict:
    done = subprocess.run([binary, *argv], env=env, capture_output=True, text=True, timeout=30)
    try:
        return json.loads(done.stdout)
    except ValueError:
        return {"ok": False, "raw": done.stdout[-400:], "stderr": done.stderr[-400:], "rc": done.returncode}


def nodes(value):
    if isinstance(value, dict):
        yield value
        value = list(value.values())
    if isinstance(value, list):
        for item in value:
            if isinstance(item, (dict, list)):
                yield from nodes(item)


def target_ref(binary: str, env: dict[str, str], pid: int) -> str | None:
    windows = product(binary, env, "list-windows").get("data") or []
    ids = [item.get("id") for item in windows if item.get("pid") == pid]
    if len(ids) != 1:
        return None
    found = product(binary, env, "find", "--exact", "--limit", "0", "--role", "button", "--name",
                    "Probe Target", "--window-id", ids[0])
    refs = {item["ref_id"] for item in nodes(found.get("data")) if isinstance(item.get("ref_id"), str)}
    return refs.pop() if len(refs) == 1 else None


def start_cover(case: str, hypr: Hypr, env: dict[str, str], fixture_client: dict):
    if case == "layer":
        cover = subprocess.Popen(["slurp"], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        mapped = lambda: any(entry.get("namespace") == "selection" and entry.get("pid") == cover.pid
                             for monitor in hypr.query("layers").values()
                             for entries in (monitor.get("levels") or {}).values() for entry in entries)
        return cover, {"mapped": hypr.wait_event("openlayer", "selection", 5, mapped) and mapped()}
    cover = subprocess.Popen(["foot", "--app-id", COVER_CLASS, "--title", COVER_CLASS, "sleep", "infinity"],
                             env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    hypr.wait_event("openwindow", COVER_CLASS, 8, lambda: hypr.client(cover.pid) is not None)
    client = hypr.client(cover.pid)
    if client is None:
        return cover, {"mapped": False}
    (x, y), (width, height) = fixture_client["at"], fixture_client["size"]
    replies = [hypr.window("float", client["address"]), hypr.window("resize", client["address"], width, height),
               hypr.window("move", client["address"], x, y)]
    placed = hypr.client(cover.pid) or {}
    return cover, {"mapped": True, "replies": replies, "at": placed.get("at"), "size": placed.get("size"),
                   "floating": placed.get("floating")}


def stop(process: subprocess.Popen) -> None:
    if process.poll() is None:
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=5)


def one_run(binary: str, env: dict[str, str], hypr: Hypr, atspi: Atspi, toolkit: str, case: str) -> dict:
    fixture_case = case if case in ("overlay", "popup") else "plain"
    app = subprocess.Popen([sys.executable, "-I", __file__, "fixture", toolkit, fixture_case], env=env,
                           stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
    lines = Lines(app.stdout)
    result: dict = {"toolkit": toolkit, "case": case}
    cover = None
    try:
        lines.until(lambda seen: any(item.get("ev") == "mapped" for item in seen), 15)
        hypr.wait_event("openwindow", TITLE, 5, lambda: hypr.client(app.pid) is not None)
        client = hypr.client(app.pid)
        result["fixture_client"] = client and {key: client.get(key) for key in ("at", "size", "floating",
                                                                                 "xwayland")}
        if client is None:
            return {**result, "verdict": "setup_failed", "why": "fixture never mapped"}
        if case == "popup":
            lines.until(lambda seen: any(item.get("ev") == "popup-shown" for item in seen), 5)
            result["popup_shown"] = any(item.get("ev") == "popup-shown" for item in lines.seen)
        if case in ("layer", "float"):
            cover, result["cover"] = start_cover(case, hypr, env, client)
        ref = target_ref(binary, env, app.pid)
        if ref is None:
            return {**result, "verdict": "setup_failed", "why": "no unique ref for Probe Target"}
        root = atspi.app(app.pid)
        result["toplevels"] = [atspi.describe(node) for node in atspi.children(root)] if root else None
        frames = [node for node in atspi.children(root) if atspi.describe(node)["role"] == "frame"] if root else []
        target = atspi.named(root, "Probe Target") if root else None
        if target and frames:
            described = atspi.describe(target)
            result["target"] = described
            wx, wy, ww, wh = described.get("window", (0, 0, 0, 0))
            sx, sy, sw, sh = described.get("screen", (0, 0, 0, 0))
            result["descent_window"] = atspi.descend(frames[0], (wx + ww // 2, wy + wh // 2), 1)
            result["descent_screen"] = atspi.descend(frames[0], (sx + sw // 2, sy + sh // 2), 0)
        result["product_bounds"] = product(binary, env, "get", ref, "--property", "bounds").get("data")
        result["actionable"] = product(binary, env, "--headed", "wait", "--element", ref, "--predicate",
                                       "actionable", "--action", "click", "--timeout", "1500")
        result["layers"] = sorted({entry.get("namespace") for monitor in hypr.query("layers").values()
                                   for level, entries in (monitor.get("levels") or {}).items() if int(level) >= 2
                                   for entry in entries})
        mark = len(lines.seen)
        result["click"] = product(binary, env, "--headed", "click", ref)
        settle = 1.5 if result["click"].get("ok") else 0.8
        seen = lines.until(lambda all_: any(item.get("ev") == "clicked" for item in all_[mark:]), settle)[mark:]
        result["effects"] = [item for item in seen if item.get("ev") in ("clicked", "popup-closed")]
        clicked = [item["widget"] for item in result["effects"] if item.get("ev") == "clicked"]
        ok = bool(result["click"].get("ok"))
        result["verdict"] = (("hit" if clicked == ["Probe Target"] else "false_success") if ok
                             else ("refused" if not clicked else "refused_with_effect"))
        return result
    finally:
        if cover is not None:
            stop(cover)
        stop(app)


def run(out: Path) -> int:
    if not os.environ.get("OMARCHY_VM_CHANNEL"):
        sys.exit("hyprland_hit_probe: run this inside an Omarchy VM, never on a host")
    out.mkdir(parents=True, exist_ok=True)
    home = tempfile.mkdtemp(prefix="ad-hit-probe-")
    env = dict(os.environ, AGENT_DESKTOP_HOME=home, GDK_BACKEND="wayland", QT_QPA_PLATFORM="wayland",
               QT_ACCESSIBILITY="1", QT_LINUX_ACCESSIBILITY_ALWAYS_ON="1", GTK_USE_PORTAL="0")
    binary = os.environ.get("AGENT_DESKTOP_E2E_BIN", str(Path.home() / "src/agent-desktop-bin/agent-desktop"))
    hypr, atspi = Hypr(env), Atspi()
    rows = []
    with (out / "results.jsonl").open("w") as stream:
        for toolkit in TOOLKITS:
            for case in CASES:
                for round_ in range(ROUNDS):
                    try:
                        row = one_run(binary, env, hypr, atspi, toolkit, case)
                    except Exception as error:
                        row = {"toolkit": toolkit, "case": case, "verdict": "probe_error", "why": repr(error)[:300]}
                    row["round"] = round_
                    stream.write(json.dumps(row) + "\n")
                    stream.flush()
                    rows.append(row)
    for row in rows:
        click = row.get("click") or {}
        error = click.get("error") or {}
        details = error.get("details") or {}
        lander = (row.get("descent_window") or [{}])[-1] if row.get("descent_window") else {}
        print(f"{row['toolkit']:5} {row['case']:8} r{row['round']} {row['verdict']:20} "
              f"code={error.get('code', '-')} cause={details.get('cause', details.get('kind', '-'))} "
              f"effects={[item.get('widget', item.get('ev')) for item in row.get('effects', [])]} "
              f"descent1={lander.get('role')}/{lander.get('name')} "
              f"toplevels={[(item['role'], item['showing']) for item in row.get('toplevels') or []]}")
    return 0


if __name__ == "__main__":
    if sys.argv[1:2] == ["fixture"]:
        fixture(sys.argv[2], sys.argv[3])
    elif sys.argv[1:2] == ["run"] and len(sys.argv) == 3:
        sys.exit(run(Path(sys.argv[2]).expanduser()))
    else:
        sys.exit(__doc__)
